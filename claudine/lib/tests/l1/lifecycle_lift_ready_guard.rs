//! The lifecycle parser and action-model slice names no Claudine runtime
//! type (AC9).
//!
//! `parse.rs`, `action_shape.rs`, `actions.rs`, `validate.rs`,
//! `signatures.rs`, and `source_map.rs` under
//! `darkmatter/lib/src/lifecycle/` moved into Darkmatter as a unit, and the
//! lifecycle shell walk (`preflight.rs`, `preflight/shell_walk.rs`,
//! `preflight/candidates.rs`) followed it, so none
//! of them may name a Claudine runtime type: global settings, the
//! configuration layer, the provider layer, messaging, or TTS/Playa. The
//! effect catalog the parser checks `effect` names against is supplied by its
//! caller instead. Any textual reference fails, a comment included.

/// Every name the slice must not mention.
const FORBIDDEN: &[&str] = &[
    "crate::config",
    "crate::events",
    "crate::provider",
    "crate::messaging",
    "GlobalSettings",
    "TtsSettings",
    "TtsConfig",
    "biscuit_speaks",
    "playa",
];

/// Each forbidden name a slice file mentions, with its line number.
fn forbidden_references(source: &str) -> Vec<(usize, &'static str)> {
    source
        .lines()
        .enumerate()
        .flat_map(|(index, line)| {
            FORBIDDEN
                .iter()
                .filter(move |name| line.contains(*name))
                .map(move |name| (index + 1, *name))
        })
        .collect()
}

fn assert_lift_ready(file: &str, source: &str) {
    let found = forbidden_references(source);
    assert!(
        found.is_empty(),
        "lifecycle/{file} must stay lift-ready, but names Claudine runtime types: {found:?}"
    );
}

#[test]
fn parse_names_no_runtime_type() {
    assert_lift_ready(
        "parse.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/parse.rs"),
    );
}

#[test]
fn action_shape_names_no_runtime_type() {
    assert_lift_ready(
        "action_shape.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/action_shape.rs"),
    );
}

#[test]
fn actions_names_no_runtime_type() {
    assert_lift_ready(
        "actions.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/actions.rs"),
    );
}

#[test]
fn validate_names_no_runtime_type() {
    assert_lift_ready(
        "validate.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/validate.rs"),
    );
}

#[test]
fn signatures_names_no_runtime_type() {
    assert_lift_ready(
        "signatures.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/signatures.rs"),
    );
}

#[test]
fn source_map_names_no_runtime_type() {
    assert_lift_ready(
        "source_map.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/source_map.rs"),
    );
}

#[test]
fn preflight_names_no_runtime_type() {
    assert_lift_ready(
        "preflight.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/preflight.rs"),
    );
}

#[test]
fn preflight_shell_walk_names_no_runtime_type() {
    assert_lift_ready(
        "preflight/shell_walk.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/preflight/shell_walk.rs"),
    );
}

#[test]
fn preflight_candidates_names_no_runtime_type() {
    assert_lift_ready(
        "preflight/candidates.rs",
        include_str!("../../../../darkmatter/lib/src/lifecycle/preflight/candidates.rs"),
    );
}

/// The scan itself must detect each name, or a passing guard proves nothing.
#[test]
fn the_scan_detects_every_forbidden_name() {
    for name in FORBIDDEN {
        let source = format!("fn ok() {{}}\nuse {name}::Thing;\n");
        assert_eq!(forbidden_references(&source), vec![(2, *name)]);
    }
}

#[test]
fn criterion_14_deliberate_runtime_import_fails_the_guard() {
    let rejected = std::panic::catch_unwind(|| {
        assert_lift_ready("parse.rs", "use crate::config::GlobalSettings;");
    });
    assert!(rejected.is_err(), "a runtime import must make the guard fail");
}
