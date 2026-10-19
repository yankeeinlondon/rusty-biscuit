//! The trigger envelope claim rule for source that is not valid YAML, read
//! through `parse_trigger_envelope_from_str`, the classifier every caller
//! (discovery, embedded bundles, DMLS authoring, payload-cycle checks) shares.
//!
//! Default outcome: source that fails to parse is a
//! [`SchemaError::TriggerEnvelopeYaml`] when a provable top-level
//! `kind: trigger-schema` line claims it, and `Ok(None)` otherwise. Parseable
//! source reads the claim from the parsed mapping, exactly as before.

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::schemas::{SchemaError, TriggerEnvelope, parse_trigger_envelope_from_str};

/// A valid envelope; every unparseable cell appends [`BROKEN`] or edits it.
const CONTROL: &str = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema:\n  model: string\n";

/// A line that makes any document invalid YAML (`: ` inside a plain value).
const BROKEN: &str = "notes: a value: with a colon\n";

fn classify(source: &str) -> Result<Option<TriggerEnvelope>, SchemaError> {
    parse_trigger_envelope_from_str(source)
}

/// `(line, column)` of a claimed, unparseable envelope's YAML error.
fn yaml_error(source: &str) -> (Option<usize>, Option<usize>, String) {
    match classify(source) {
        Err(SchemaError::TriggerEnvelopeYaml { message, line, column }) => (line, column, message),
        other => panic!("expected a YAML envelope error for {source:?}, got {other:?}"),
    }
}

fn render(error: &SchemaError) -> String {
    let terminal = biscuit_terminal::terminal::Terminal::default();
    strip_escape_codes(error.status_block(&terminal).render_optimistic(Some(200)))
}

#[test]
fn the_control_envelope_loads() {
    let envelope = classify(CONTROL).expect("control").expect("claimed");
    assert_eq!(envelope.match_arms.0.len(), 1);
    assert!(envelope.payload.is_some());
}

#[test]
fn claim_lines_in_unparseable_source_are_yaml_errors() {
    for claim in [
        "kind: trigger-schema",
        "kind: 'trigger-schema'",
        "kind: \"trigger-schema\"",
        "kind: \"trigger-schema\"  # note",
        "kind: trigger-schema # note",
        "kind : trigger-schema",
        "kind:\ttrigger-schema",
    ] {
        let source = format!("{claim}\nmatch:\n  prompt: string(required)\n{BROKEN}");
        let (line, column, message) = yaml_error(&source);
        assert_eq!((line, column), (Some(4), Some(15)), "{claim:?}: {message}");
        assert_eq!(message, "mapping values are not allowed in this context", "{claim:?}");
    }
}

#[test]
fn unclaimed_unparseable_source_is_not_a_trigger() {
    let cells = [
        ("no kind line", format!("match:\n  prompt: string(required)\n{BROKEN}")),
        ("kind: schema", format!("kind: schema\ntypes:\n  x: string\n{BROKEN}")),
        ("indented kind", format!("meta:\n  kind: trigger-schema\n{BROKEN}")),
        ("sequence item kind", format!("items:\n- kind: trigger-schema\n{BROKEN}")),
        ("suffix", format!("kind: trigger-schema-extra\n{BROKEN}")),
        ("plain continuation", format!("kind: trigger-schema extra\n{BROKEN}")),
        ("comment without a space", format!("kind: trigger-schema#x\n{BROKEN}")),
        ("mismatched quotes", format!("kind: \"trigger-schema'\n{BROKEN}")),
        ("commented-out claim", format!("# kind: trigger-schema\n{BROKEN}")),
        ("no space after the colon", format!("kind:trigger-schema\n{BROKEN}")),
        ("other key", format!("kinds: trigger-schema\n{BROKEN}")),
        (
            "inside a multi-line double-quoted scalar",
            format!("description: \"starts here\nkind: trigger-schema\nends here\"\n{BROKEN}"),
        ),
        (
            "inside a multi-line single-quoted scalar",
            format!("description: 'it''s open\nkind: trigger-schema\nclosed'\n{BROKEN}"),
        ),
        (
            "after an escaped quote inside a double-quoted scalar",
            format!("description: \"a \\\" b\nkind: trigger-schema\nc\"\n{BROKEN}"),
        ),
        (
            "inside a block scalar",
            format!("description: |\n  kind: trigger-schema\n  more text\n{BROKEN}"),
        ),
        ("inside an open flow mapping", format!("match: {{a: 1,\nkind: trigger-schema\n}}\n{BROKEN}")),
        ("inside a root block scalar", format!("--- |\nkind: trigger-schema\n{BROKEN}")),
        ("malformed flow mapping", "{\"kind\": \"trigger-schema\", \"match\": {]}\n".to_string()),
    ];
    for (cell, source) in cells {
        assert!(serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&source).is_err(), "{cell}: must be unparseable");
        assert!(matches!(classify(&source), Ok(None)), "{cell}: {:?}", classify(&source));
    }
}

/// A claim after a closed multi-line scalar or a finished block scalar is
/// provably top-level again.
#[test]
fn a_claim_after_a_closed_scalar_still_claims() {
    for source in [
        format!("description: \"one\ntwo\"\nkind: trigger-schema\n{BROKEN}"),
        format!("description: |\n  kind: trigger-schema\n\nkind: trigger-schema\n{BROKEN}"),
        format!("plain: don't\nkind: trigger-schema\n{BROKEN}"),
        format!("match: {{a: 1}}\nkind: trigger-schema\n{BROKEN}"),
    ] {
        yaml_error(&source);
    }
}

#[test]
fn crlf_and_a_bom_classify_like_lf_without_a_bom() {
    let unclaimed = format!("kind: schema\n{BROKEN}");
    let claimed = format!("kind: trigger-schema\nmatch:\n  prompt: string(required)\n{BROKEN}");
    for (label, transform) in [
        ("crlf", (|s: &str| s.replace('\n', "\r\n")) as fn(&str) -> String),
        ("bom", |s: &str| format!("\u{feff}{s}")),
        ("bom + crlf", |s: &str| format!("\u{feff}{}", s.replace('\n', "\r\n"))),
    ] {
        assert_eq!(classify(&transform(CONTROL)).ok().flatten(), classify(CONTROL).ok().flatten(), "{label}");
        assert!(classify(&transform(CONTROL)).unwrap().is_some(), "{label}: control");
        assert!(matches!(classify(&transform(&unclaimed)), Ok(None)), "{label}: unclaimed");
        assert_eq!(yaml_error(&transform(&claimed)), yaml_error(&claimed), "{label}: claimed");
    }
}

/// The YAML parser rejects a duplicate key, so the claim rule decides.
#[test]
fn a_duplicate_kind_key_is_decided_by_the_claim_rule() {
    let claimed = "kind: schema\nkind: trigger-schema\nmatch:\n  prompt: string(required)\n";
    let (_, _, message) = yaml_error(claimed);
    assert!(message.contains("duplicate entry with key \"kind\""), "{message}");
    assert!(matches!(classify("kind: schema\nkind: schema\n"), Ok(None)));
}

#[test]
fn trailing_invalid_content_is_decided_by_the_claim_rule() {
    let (line, _, _) = yaml_error(&format!("{CONTROL}extra: [unclosed\n"));
    assert_eq!(line, Some(7));
    assert!(matches!(classify("kind: schema\ntypes: {}\nextra: [unclosed\n"), Ok(None)));
}

#[test]
fn a_second_document_is_decided_by_the_claim_rule() {
    let (_, _, message) = yaml_error(&format!("{CONTROL}---\nkind: schema\n"));
    assert!(message.contains("more than one document"), "{message}");
    assert!(matches!(classify("kind: schema\n---\nkind: schema\n"), Ok(None)));
}

#[test]
fn parseable_quoted_key_and_json_envelopes_keep_their_behavior() {
    for source in [
        "\"kind\": trigger-schema\nmatch:\n  prompt: string(required)\n",
        "'kind': 'trigger-schema'\nmatch:\n  prompt: string(required)\n",
        "{\"kind\": \"trigger-schema\", \"match\": {\"prompt\": \"string(required)\"}}\n",
    ] {
        assert!(classify(source).expect(source).is_some(), "{source}");
    }
    // A parseable claim with a bad shape stays a match-grammar error.
    assert!(matches!(
        classify("{\"kind\": \"trigger-schema\"}\n"),
        Err(SchemaError::TriggerMatch { .. })
    ));
}

/// `kind` field shapes on a parseable document: none of them claim.
#[test]
fn parseable_kind_shapes_other_than_the_claim_are_unclaimed() {
    for kind in ["", "kind: null\n", "kind:\n", "kind: 123\n", "kind: \"\"\n", "kind: [trigger-schema]\n"] {
        let source = format!("{kind}match:\n  prompt: string(required)\n");
        assert!(matches!(classify(&source), Ok(None)), "{kind:?}");
    }
}

/// The 2026-10-17 incident: `schemas/feature-review.yaml` mid-edit, an
/// ordinary schema file whose line 17 lost the quotes around a value holding
/// `): ` at column 93.
#[test]
fn the_incident_file_is_not_a_trigger() {
    let source = include_str!("../fixtures/triggers/feature-review-incident.yaml");
    let error = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).expect_err("the fixture is invalid YAML");
    let location = error.location().expect("located");
    assert_eq!((location.line(), location.column()), (17, 93), "{error}");
    assert!(matches!(classify(source), Ok(None)));
}

#[test]
fn the_yaml_error_names_yaml_not_the_match_grammar() {
    let error = classify(&format!("kind: trigger-schema\n{BROKEN}")).expect_err("claimed");
    assert_eq!(
        error.to_string(),
        "trigger-schema envelope is not valid YAML at line 2, column 15: mapping values are not allowed in this context"
    );
    let rendered = render(&error);
    assert!(rendered.contains("not valid YAML"), "{rendered}");
    assert!(rendered.contains("must be quoted"), "{rendered}");
    for grammar in ["match expression", "combinators", "min-match", "$path"] {
        assert!(!rendered.contains(grammar), "`{grammar}` in {rendered}");
    }

    let load = SchemaError::TriggerLoad { path: "/repo/schemas/x.yaml".into(), source: Box::new(error) };
    assert_eq!(
        load.to_string(),
        "trigger-schema file `/repo/schemas/x.yaml` is not valid YAML at line 2, column 15: mapping values are not \
         allowed in this context"
    );
    let rendered = render(&load);
    assert!(rendered.contains("is not valid YAML") && rendered.contains("must be quoted"), "{rendered}");
    for grammar in ["match grammar", "combinators", "min-match", "$path", "malformed"] {
        assert!(!rendered.contains(grammar), "`{grammar}` in {rendered}");
    }

    let embedded = SchemaError::EmbeddedTriggerLoad {
        bundle: "b".into(),
        name: "n.yaml".into(),
        source: Box::new(classify(&format!("kind: trigger-schema\n{BROKEN}")).expect_err("claimed")),
    };
    assert_eq!(
        embedded.to_string(),
        "embedded trigger-schema `b/n.yaml` is not valid YAML at line 2, column 15: mapping values are not \
         allowed in this context"
    );
}

/// A grammar failure keeps its grammar wording and hint.
#[test]
fn a_match_grammar_failure_keeps_its_grammar_hint() {
    let error = classify("kind: trigger-schema\n$schema: x.yaml\n").expect_err("missing match");
    assert!(matches!(error, SchemaError::TriggerMatch { .. }));
    let load = SchemaError::TriggerLoad { path: "/repo/schemas/x.yaml".into(), source: Box::new(error) };
    assert!(load.to_string().contains("is malformed: invalid trigger-schema match expression"), "{load}");
    let rendered = render(&load);
    assert!(rendered.contains("match grammar") && !rendered.contains("must be quoted"), "{rendered}");
}

/// The column counts characters: a multi-byte character before the error moves
/// the byte offset but not the column.
#[test]
fn the_reported_column_counts_characters() {
    let (_, ascii, _) = yaml_error("kind: trigger-schema\nnote: ab x: y\n");
    let (_, wide, _) = yaml_error("kind: trigger-schema\nnote: é😀 x: y\n");
    assert_eq!(ascii, wide);
    assert_eq!(ascii, Some(11));
}
