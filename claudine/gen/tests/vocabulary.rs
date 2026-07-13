//! Loader tests over the REAL committed facts inputs (Phase A seed).
//!
//! These prove the seeded `error_vocabulary` in every parser-backed
//! provider's facts file parses through the vocabulary loader with its bucket
//! order and repeated kinds intact — and, because needles are typed `String`,
//! that an accidentally-unquoted numeric needle (e.g. `503` parsing as an
//! integer) is a hard parse failure rather than silent data loss.

use std::path::Path;

use claudine_gen::inputs::load;
use claudine_gen::{build_vocabulary, load_error_vocabulary};

/// The claudine package-area root (parent of this crate's manifest dir).
fn area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

/// Every provider that owns a structured stream parser (Kilo shares
/// OpenCode's wire parser but carries its own seeded vocabulary).
const SEEDED: &[&str] = &[
    "claude", "codex", "gemini", "opencode", "kilo", "pi", "qwen", "antigravity", "kimi",
];

#[test]
fn every_seeded_facts_vocabulary_parses_with_string_needles() {
    for slug in SEEDED {
        let inputs = load(area(), slug, &[]).unwrap_or_else(|err| panic!("load `{slug}`: {err}"));
        let vocab = load_error_vocabulary(&inputs)
            .unwrap_or_else(|err| panic!("parse `{slug}` vocabulary: {err}"))
            .unwrap_or_else(|| panic!("`{slug}` facts carry no error_vocabulary"));
        assert!(
            !vocab.msg_buckets.is_empty(),
            "`{slug}`: a parser-backed provider must seed a non-empty message vocabulary"
        );
        for bucket in vocab.kind_buckets.iter().chain(&vocab.msg_buckets) {
            assert!(
                !bucket.needles.is_empty(),
                "`{slug}`: every seeded bucket must carry at least one needle"
            );
        }
    }
}

#[test]
fn kimi_seed_carries_the_complete_jsonrpc_code_mapping() {
    let inputs = load(area(), "kimi", &[]).unwrap();
    let vocab = load_error_vocabulary(&inputs).unwrap().unwrap();
    // AUTH_EXPIRED, CHAT_PROVIDER_ERROR, then the five standard JSON-RPC codes.
    let codes: Vec<i64> = vocab.code_buckets.iter().map(|c| c.code).collect();
    assert_eq!(codes, [-32004, -32005, -32700, -32600, -32601, -32602, -32603]);
    assert_eq!(vocab.code_buckets[0].kind, "configuration");
    assert_eq!(vocab.code_buckets[1].kind, "api_remote");
    assert!(vocab.code_buckets[2..].iter().all(|c| c.kind == "agent_native"));
}

#[test]
fn build_vocabulary_is_deterministic_and_well_formed() {
    let first = build_vocabulary(area()).expect("vocabulary generation must succeed");
    let second = build_vocabulary(area()).expect("vocabulary generation must succeed");
    assert_eq!(first, second, "build_vocabulary must be byte-deterministic");
    assert!(first.ends_with('\n') && !first.ends_with("\n\n"));
    // Goose (parserless) emits an explicitly empty table; Kimi carries the
    // numeric mapping.
    assert!(first.contains("Provider::Goose => &GOOSE_VOCABULARY,"));
    assert!(first.contains("(-32004, SemanticErrorKind::Configuration)"));
}

#[test]
fn kilo_seed_is_a_verbatim_ordered_copy_of_opencode() {
    let opencode = load_error_vocabulary(&load(area(), "opencode", &[]).unwrap())
        .unwrap()
        .unwrap();
    let kilo = load_error_vocabulary(&load(area(), "kilo", &[]).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        opencode, kilo,
        "Kilo's Phase A seed must be a verbatim ordered copy of OpenCode's table"
    );
}
