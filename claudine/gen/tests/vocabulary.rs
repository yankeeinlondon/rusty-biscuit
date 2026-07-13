//! Loader tests over the REAL graduated `agent-errors` research inputs.
//!
//! These prove every parser-backed provider projects provenance-bearing
//! frontmatter through the runtime vocabulary loader with bucket order and
//! repeated kinds intact.

use std::path::Path;

use claudine_gen::inputs::load;
use claudine_gen::{RESEARCH_TOPIC, build_vocabulary, load_error_vocabulary};

/// The claudine package-area root (parent of this crate's manifest dir).
fn area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

/// Every provider that owns a structured stream parser (Kilo shares
/// OpenCode's wire parser but carries its own researched vocabulary).
const PARSER_BACKED: &[&str] = &[
    "claude", "codex", "gemini", "opencode", "kilo", "pi", "qwen", "antigravity", "kimi",
];

#[test]
fn every_research_vocabulary_projects_to_runtime_strings() {
    for slug in PARSER_BACKED {
        let inputs = load(area(), slug, &[RESEARCH_TOPIC])
            .unwrap_or_else(|err| panic!("load `{slug}`: {err}"));
        let vocab = load_error_vocabulary(&inputs)
            .unwrap_or_else(|err| panic!("parse `{slug}` vocabulary: {err}"))
            .unwrap_or_else(|| panic!("`{slug}` research carries no runtime vocabulary"));
        assert!(
            !vocab.msg_buckets.is_empty(),
            "`{slug}`: a parser-backed provider must seed a non-empty message vocabulary"
        );
        for bucket in vocab.kind_buckets.iter().chain(&vocab.msg_buckets) {
            assert!(
                !bucket.needles.is_empty(),
                "`{slug}`: every projected bucket must carry at least one needle"
            );
        }
    }
}

#[test]
fn kimi_research_projects_the_complete_jsonrpc_code_mapping() {
    let inputs = load(area(), "kimi", &[RESEARCH_TOPIC]).unwrap();
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
fn kilo_research_is_a_verbatim_ordered_copy_of_opencode() {
    let opencode = load_error_vocabulary(
        &load(area(), "opencode", &[RESEARCH_TOPIC]).unwrap(),
    )
        .unwrap()
        .unwrap();
    let kilo = load_error_vocabulary(&load(area(), "kilo", &[RESEARCH_TOPIC]).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        opencode, kilo,
        "Kilo's graduated research must remain an ordered copy of OpenCode's table"
    );
}
