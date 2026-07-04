//! Drift and byte-equivalence tests over the REAL committed inputs.
//!
//! The drift test and the CLI `check` subcommand are the same code path:
//! both call [`claudine_gen::check_area`].

use std::path::Path;

use claudine_gen::{CheckOutcome, check_area, generate_for_area};

/// The claudine package-area root (parent of this crate's manifest dir).
fn area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

/// Gate (d): generate(committed inputs) == committed fragment file.
#[test]
fn committed_fragment_matches_regenerated_inputs() {
    let (_, outcome) = check_area(area(), "claude").expect("generation must succeed");
    match outcome {
        CheckOutcome::Clean => {}
        CheckOutcome::Drift { details } => panic!(
            "drift between committed inputs and gen/generated/claude.data.rs — \
             run `claudine-gen generate`:\n{}",
            details.join("\n")
        ),
        CheckOutcome::MissingCommitted { path } => panic!(
            "committed fragment missing at {} — run `claudine-gen generate`",
            path.display()
        ),
    }
}

/// Exit criterion 1: every generated field expression is byte-identical to
/// the hand-written text in `lib/src/provider/claude.rs`.
///
/// Matching approach: the generated fragment emits one
/// `    field: expr,` line per registry entry at the same four-space
/// indentation the `CLAUDE_INFO` initializer uses, so each non-comment
/// fragment line must appear VERBATIM (full-line equality, indentation
/// included) inside the `CLAUDE_INFO = ProviderInfo { ... }` block.
#[test]
fn generated_field_expressions_byte_match_hand_written_claude_rs() {
    let generation = generate_for_area(area(), "claude").expect("generation must succeed");
    let claude_rs = std::fs::read_to_string(area().join("lib/src/provider/claude.rs"))
        .expect("hand-written claude.rs must exist");
    let block = claude_rs
        .split("static CLAUDE_INFO: ProviderInfo = ProviderInfo {")
        .nth(1)
        .expect("CLAUDE_INFO initializer present")
        .split("\n};")
        .next()
        .expect("CLAUDE_INFO initializer terminated");

    let field_lines: Vec<&str> = generation
        .fragment
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("//"))
        .collect();
    assert_eq!(
        field_lines.len(),
        claudine_gen::REGISTRY.len(),
        "one generated line per registry entry"
    );
    for line in field_lines {
        assert!(
            block.lines().any(|hand_written| hand_written == line),
            "generated `{line}` has no byte-identical line in CLAUDE_INFO"
        );
    }
}
