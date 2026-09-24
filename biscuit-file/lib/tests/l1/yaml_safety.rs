//! Safety invariants over a shared corpus (acceptance C-1, C-2, C-4):
//!
//! - accepted S1 edits reparse to a value exactly equal to the original's;
//! - S3 repairs satisfy the parse-recovery proof (lexeme byte-equality at the
//!   scanned context);
//! - every byte outside an accepted edit is preserved, reconstructing the
//!   output solely from accepted spans.

use biscuit_file::{EditSetOutcome, YamlCertainty, analyze_yaml};

fn value_of(source: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(source).expect("source must parse")
}

fn assert_untouched_bytes_preserved(original: &str, outcome: &EditSetOutcome) {
    let mut expected = String::with_capacity(outcome.source.len());
    let mut cursor = 0;
    for repair in &outcome.audit.applied {
        expected.push_str(&original[cursor..repair.span.start]);
        expected.push_str(&repair.replacement);
        cursor = repair.span.end;
    }
    expected.push_str(&original[cursor..]);
    assert_eq!(
        outcome.source, expected,
        "output must be reconstructable from accepted spans and original bytes only"
    );
}

/// Parseable inputs exercising every S1 repair class.
const S1_CORPUS: &[&str] = &[
    "\u{FEFF}key: value\n",
    "a: 1\r\nb: 2\r\n",
    "a: 1\rb: 2\r",
    "a: 1\r\nb: 2\rc: 3\n",
    "key: value  \n",
    "key: value\t\n",
    "a: 1\n  \nb: 2\n",
    "key: value",
    "key: value\n\n\n",
    "key :  value",
    "-  item",
    "key: [ 80,443 ]",
    "{ a:  1 , b: 2 }\n",
    "{\"a\":1}\n",
    "[ [ 1 ], {a:  2} ]\n",
    "[\"a b\" , \"c\" ]\n",
    "[http://x,  y]\n",
    "key:\tvalue\n",
    "\u{FEFF}key :  [ 80,443 ]  \r\n",
    "script: |\r\n  line1\r\n  line2\r\n",
    "key: 日本語\r\nother: 1\r\n",
    "multi: \"a  b\"\nlist: [1,  2]\n",
];

/// Unparseable inputs exercising S3 reserved-indicator repair:
/// (source, expected output).
const S3_CORPUS: &[(&str, &str)] = &[
    ("title: @daily-report", "title: \"@daily-report\""),
    ("title: @daily-report\n", "title: \"@daily-report\"\n"),
    ("title: `daily-report`", "title: \"`daily-report`\""),
    ("title: %directive", "title: \"%directive\""),
    ("title: &&x", "title: \"&&x\""),
    ("title: *undefined", "title: \"*undefined\""),
    ("title: |foo", "title: \"|foo\""),
    ("title: >foo", "title: \">foo\""),
    ("title: ,foo", "title: \",foo\""),
    ("title: ]foo", "title: \"]foo\""),
    ("items:\n  - one\n  - @two\n", "items:\n  - one\n  - \"@two\"\n"),
    ("a:\n  b:\n    title: @x\n", "a:\n  b:\n    title: \"@x\"\n"),
    ("title: @a\ndate: `b\n", "title: \"@a\"\ndate: \"`b\"\n"),
    ("title: @日本語", "title: \"@日本語\""),
    ("title: @x\r\n", "title: \"@x\"\r\n"),
];

/// Inputs that must come through untouched: report-only boundaries and
/// clean documents.
const UNTOUCHED_CORPUS: &[&str] = &[
    "title: @foo # comment",
    "title: @daily\n  report\n",
    "data: {@x: 1}",
    "title: 'unterminated",
    "title: \"unterminated",
    "host:localhost\n",
    "ports:\n  -80\n  -443\n",
    "token: abc #123\n",
    "url: http://example.com\n",
    "path: C:\\Users\\Ken\n",
    "key: value\n",
    "anchor: &x 1\nalias: *x\n",
    "script: |\n  echo hi  \n  echo bye\n",
    "key: [1, # c\n 2]\n",
    "empty:\n",
];

#[test]
fn s1_accepted_edits_preserve_values() {
    for source in S1_CORPUS {
        let analysis = analyze_yaml(source);
        assert!(analysis.is_parseable(), "corpus input must parse: {source:?}");
        let outcome = analysis.apply();
        assert_eq!(
            value_of(source),
            value_of(&outcome.source),
            "accepted edits must preserve the parsed value for {source:?}"
        );
    }
}

#[test]
fn s1_repairs_are_deterministic_across_runs() {
    for source in S1_CORPUS {
        let first = analyze_yaml(source).apply();
        let second = analyze_yaml(source).apply();
        assert_eq!(first.source, second.source, "for {source:?}");
    }
}

#[test]
fn s3_repairs_satisfy_parse_recovery_proof() {
    for (source, expected) in S3_CORPUS {
        let analysis = analyze_yaml(source);
        assert!(!analysis.is_parseable(), "corpus input must not parse: {source:?}");
        let outcome = analysis.apply();
        assert_eq!(&outcome.source, expected, "for {source:?}");
        // The repaired document parses, and every deterministic diagnostic's
        // lexeme lands as a string byte-equal to the original lexeme.
        let repaired = value_of(&outcome.source);
        for diagnostic in analysis
            .diagnostics()
            .iter()
            .filter(|d| d.classification == YamlCertainty::Deterministic)
        {
            let lexeme = &source[diagnostic.span.clone()];
            assert!(
                contains_string(&repaired, lexeme),
                "repaired value must contain {lexeme:?} for {source:?}"
            );
        }
    }
}

#[test]
fn untouched_bytes_preserved_across_corpus() {
    for source in S1_CORPUS.iter().chain(S3_CORPUS.iter().map(|(source, _)| source)) {
        let outcome = analyze_yaml(source).apply();
        assert_untouched_bytes_preserved(source, &outcome);
    }
}

#[test]
fn report_only_and_clean_inputs_stay_byte_identical() {
    for source in UNTOUCHED_CORPUS {
        let outcome = analyze_yaml(source).apply();
        assert_eq!(
            &outcome.source, source,
            "no automatic edit expected for {source:?}"
        );
        assert!(!outcome.changed());
    }
}

fn contains_string(value: &serde_yaml_ng::Value, expected: &str) -> bool {
    use serde_yaml_ng::Value;
    match value {
        Value::String(text) => text == expected,
        Value::Sequence(sequence) => sequence.iter().any(|item| contains_string(item, expected)),
        Value::Mapping(mapping) => mapping.values().any(|item| contains_string(item, expected)),
        _ => false,
    }
}
