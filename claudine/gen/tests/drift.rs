//! Drift tests over the REAL committed inputs.
//!
//! The drift test and the CLI `check` subcommand are the same code path:
//! both call [`claudine_gen::check_area`]. For every provider slug,
//! `generate(committed inputs)` must byte-equal the committed
//! `lib/src/provider/<slug>/data.rs`.

use std::path::Path;

use biscuit_hash::xx_hash_bytes;
use claudine_gen::{
    CheckOutcome, check_area, check_catalog, check_families, check_signals, check_vocabulary,
    generate_all, provider_slugs,
};

/// The claudine package-area root (parent of this crate's manifest dir).
fn area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

/// Phase 1 byte baseline for every committed artifact owned by the provider
/// generator. The normal drift tests prove regeneration convergence; this
/// inventory separately records the exact pre-refactor bytes.
///
/// The baseline is a permanent pin consumed by this test, so it lives with the
/// crate's fixtures rather than under `reviews/` — archiving a review moves
/// that tree, and the pin must not move with it.
#[test]
fn committed_generated_artifacts_match_phase_1_byte_baseline() {
    let baseline_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/generated-artifact-baseline.json");
    let raw = std::fs::read(&baseline_path).unwrap_or_else(|err| {
        panic!(
            "phase 1 artifact baseline must be readable at {}: {err}",
            baseline_path.display()
        )
    });
    let baseline: serde_json::Value =
        serde_json::from_slice(&raw).expect("phase 1 artifact baseline must be valid JSON");
    assert_eq!(baseline["algorithm"], "xxh64");
    assert_eq!(baseline["seed"], 0);
    let artifacts = baseline["artifacts"]
        .as_array()
        .expect("artifact baseline must contain an artifacts array");
    assert_eq!(artifacts.len(), 14, "every generated artifact must be pinned");

    for artifact in artifacts {
        let relative = artifact["path"]
            .as_str()
            .expect("artifact path must be a string");
        let expected = artifact["hash"]
            .as_str()
            .expect("artifact hash must be a decimal string")
            .parse::<u64>()
            .expect("artifact hash must fit u64");
        let bytes = std::fs::read(area().join(relative))
            .unwrap_or_else(|err| panic!("generated artifact `{relative}` must be readable: {err}"));
        assert_eq!(
            xx_hash_bytes(&bytes),
            expected,
            "generated artifact `{relative}` changed from the Phase 1 byte baseline"
        );
    }
}

/// generate(committed inputs) == committed lib/src/provider/<slug>/data.rs
/// for every provider.
#[test]
fn committed_data_matches_regenerated_inputs() {
    for slug in provider_slugs() {
        let (_, outcome) = check_area(area(), slug)
            .unwrap_or_else(|err| panic!("generation for `{slug}` must succeed: {err}"));
        match outcome {
            CheckOutcome::Clean => {}
            CheckOutcome::Drift { details } => panic!(
                "drift between committed inputs and lib/src/provider/{slug}/data.rs — \
                 run `claudine-gen generate`:\n{}",
                details.join("\n")
            ),
            CheckOutcome::MissingCommitted { path } => panic!(
                "committed data.rs missing at {} — run `claudine-gen generate`",
                path.display()
            ),
        }
    }
}

/// build_signals(committed signals corpus) == committed
/// lib/src/signals/generated.rs (full scope, all nine docs — including the
/// dormant kilo/pi tables).
#[test]
fn committed_signals_match_regenerated_inputs() {
    match check_signals(area()).expect("signals generation must succeed") {
        CheckOutcome::Clean => {}
        CheckOutcome::Drift { details } => panic!(
            "drift between committed inputs and lib/src/signals/generated.rs — \
             run `claudine-gen generate`:\n{}",
            details.join("\n")
        ),
        CheckOutcome::MissingCommitted { path } => panic!(
            "committed signals generated.rs missing at {} — run `claudine-gen generate`",
            path.display()
        ),
    }
}

/// build_vocabulary(committed facts) == committed
/// lib/src/stream/providers/vocabulary.rs (full scope, every wired provider —
/// Goose's explicitly empty table included).
#[test]
fn committed_vocabulary_matches_regenerated_inputs() {
    match check_vocabulary(area()).expect("vocabulary generation must succeed") {
        CheckOutcome::Clean => {}
        CheckOutcome::Drift { details } => panic!(
            "drift between committed facts and lib/src/stream/providers/vocabulary.rs — \
             run `claudine-gen generate`:\n{}",
            details.join("\n")
        ),
        CheckOutcome::MissingCommitted { path } => panic!(
            "committed vocabulary.rs missing at {} — run `claudine-gen generate`",
            path.display()
        ),
    }
}

/// build_catalog(committed inputs) == committed docs/providers/catalog.json
/// (the full-scope superset projection travels with the compiled subset).
#[test]
fn committed_catalog_matches_regenerated_inputs() {
    let generations = generate_all(area()).expect("full-scope generation must succeed");
    match check_catalog(area(), &generations).expect("catalog check must run") {
        CheckOutcome::Clean => {}
        CheckOutcome::Drift { details } => panic!(
            "drift between committed inputs and docs/providers/catalog.json — \
             run `claudine-gen generate`:\n{}",
            details.join("\n")
        ),
        CheckOutcome::MissingCommitted { path } => panic!(
            "committed catalog.json missing at {} — run `claudine-gen generate`",
            path.display()
        ),
    }
}

/// build_families(committed artifact + inputs) == committed
/// lib/src/model_catalog/families_generated.rs (the vendored family
/// slice tracks the expected-offering joins AND the artifact).
#[test]
fn committed_families_match_regenerated_inputs() {
    let generations = generate_all(area()).expect("full-scope generation must succeed");
    match check_families(area(), &generations).expect("families generation must succeed") {
        CheckOutcome::Clean => {}
        CheckOutcome::Drift { details } => panic!(
            "drift between committed inputs and lib/src/model_catalog/families_generated.rs — \
             run `claudine-gen generate`:\n{}",
            details.join("\n")
        ),
        CheckOutcome::MissingCommitted { path } => panic!(
            "committed families_generated.rs missing at {} — run `claudine-gen generate`",
            path.display()
        ),
    }
}
