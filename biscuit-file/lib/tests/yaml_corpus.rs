//! Pinned-corpus acceptance for the source-first YAML analyzer.
//!
//! The corpus lives in `tests/corpus/yaml_corpus.json` and is loaded from
//! disk at test time — never fetched. Its header documents the provenance
//! tags and the two case classes.
//!
//! Per-class expectations (`preserved` comes back byte-identical, `repaired`
//! comes back equal to `expected`) are checked alongside four invariants that
//! must hold for *every* case regardless of class:
//!
//! - analysis never panics, on parseable or unparseable input;
//! - repair is deterministic across repeated runs;
//! - repair is idempotent — the spec's fixed-point requirement;
//! - the output is reconstructable from the original bytes plus the accepted
//!   spans alone, so nothing outside an accepted edit can change.

use biscuit_file::{EditSetOutcome, analyze_yaml};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Corpus {
    yaml_test_suite: UpstreamCorpus,
    preserved: Vec<PreservedCase>,
    repaired: Vec<RepairedCase>,
}

#[derive(Debug, Deserialize)]
struct UpstreamCorpus {
    repository: String,
    release: String,
    commit: String,
    license: String,
    notice: String,
    cases: Vec<UpstreamCase>,
}

#[derive(Debug, Deserialize)]
struct UpstreamCase {
    id: String,
    name: String,
    category: String,
    source_path: String,
    source: String,
    expect_parse: Option<bool>,
    #[serde(default)]
    expect_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PreservedCase {
    name: String,
    source: String,
    #[serde(default)]
    expect_codes: Vec<String>,
    #[serde(default)]
    expect_no_diagnostics: bool,
}

#[derive(Debug, Deserialize)]
struct RepairedCase {
    name: String,
    source: String,
    expected: String,
}

fn corpus() -> Corpus {
    // `concat!` with `CARGO_MANIFEST_DIR` resolves at compile time and uses
    // the platform's own separator handling in `Path`, so this works
    // unchanged on Windows.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join("yaml_corpus.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("corpus must be readable at {}: {error}", path.display()));
    serde_json::from_str(&raw).expect("corpus must be valid JSON")
}

/// Every case in the corpus, as `(name, source)`.
fn all_cases() -> Vec<(String, String)> {
    let corpus = corpus();
    let upstream = corpus.yaml_test_suite.cases.into_iter().map(|case| {
        (
            format!("yaml-test-suite/{}", case.id),
            case.source,
        )
    });
    upstream
        .chain(
            corpus
        .preserved
        .into_iter()
        .map(|case| (case.name, case.source))
        )
        .chain(
            corpus
                .repaired
                .into_iter()
                .map(|case| (case.name, case.source)),
        )
        .collect()
}

/// Mirrors `md clean`'s frontmatter loop: analyze, apply, and — only when an
/// edit restored parseability — rescan once so the repairs that were
/// unprovable on the first pass can land. This is the sequence the shipped
/// command performs, so it is the sequence the fixed-point claim is about.
fn clean_to_fixed_point(source: &str) -> String {
    let first = analyze_yaml(source).apply().source;
    if first != source && serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&first).is_ok() {
        return analyze_yaml(&first).apply().source;
    }
    first
}

fn assert_untouched_bytes_preserved(name: &str, original: &str, outcome: &EditSetOutcome) {
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
        "[{name}] output must be reconstructable from accepted spans and original bytes only"
    );
}

// --- Class expectations ---------------------------------------------------

#[test]
fn yaml_test_suite_subset_is_release_pinned_and_preserved() {
    const REPOSITORY: &str = "https://github.com/yaml/yaml-test-suite";
    const RELEASE: &str = "data-2022-01-17";
    const COMMIT: &str = "6e6c296ae9c9d2d5c4134b4b64d01b29ac19ff6f";

    let upstream = corpus().yaml_test_suite;
    assert_eq!(upstream.repository, REPOSITORY);
    assert_eq!(upstream.release, RELEASE);
    assert_eq!(upstream.commit, COMMIT);
    assert_eq!(upstream.license, "MIT");
    assert_eq!(upstream.notice, "YAML-TEST-SUITE-NOTICE.md");

    let required_categories = [
        "valid",
        "expected-failure",
        "duplicate-key",
        "anchor-alias",
        "flow",
        "scalar",
        "multi-document",
    ];

    for category in required_categories {
        assert!(
            upstream.cases.iter().any(|case| case.category == category),
            "pinned subset is missing the {category:?} category"
        );
    }

    let mut ids: Vec<&str> = upstream.cases.iter().map(|case| case.id.as_str()).collect();
    ids.sort_unstable();
    let total = ids.len();
    ids.dedup();
    assert_eq!(total, ids.len(), "upstream case IDs must be unique");

    for case in upstream.cases {
        assert_eq!(
            case.source_path,
            format!("{}/in.yaml", case.id),
            "[{}] source path must be derivable from the release layout",
            case.id
        );

        let analysis = analyze_yaml(&case.source);
        let outcome = analysis.apply();
        assert_eq!(
            outcome.source, case.source,
            "[{} — {}] upstream bytes must be preserved",
            case.id, case.name
        );

        if let Some(expect_parse) = case.expect_parse {
            assert_eq!(
                analysis.is_parseable(),
                expect_parse,
                "[{} — {}] parse expectation",
                case.id,
                case.name
            );
        }

        let codes: Vec<&str> = analysis
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();
        for expected in case.expect_codes {
            assert!(
                codes.contains(&expected.as_str()),
                "[{} — {}] expected diagnostic {expected:?}, got {codes:?}",
                case.id,
                case.name
            );
        }
    }
}

#[test]
fn preserved_cases_are_byte_identical() {
    for case in corpus().preserved {
        let outcome = analyze_yaml(&case.source).apply();
        assert_eq!(
            outcome.source, case.source,
            "[{}] must come through byte-identical",
            case.name
        );
        assert!(
            !outcome.changed(),
            "[{}] must record no applied edit",
            case.name
        );
    }
}

/// A `preserved` case is allowed to *report*; it is only forbidden to mutate.
/// Cases that pin codes assert the finding actually fires, so a regression
/// that silently stops detecting duplicate keys is caught here rather than
/// passing as "nothing changed".
#[test]
fn preserved_cases_report_their_pinned_codes() {
    let mut failures: Vec<String> = Vec::new();

    for case in corpus().preserved {
        let analysis = analyze_yaml(&case.source);
        let codes: Vec<&str> = analysis
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        if case.expect_no_diagnostics && !analysis.is_clean() {
            failures.push(format!(
                "[{}] expected a wholly clean analysis, got {codes:?}",
                case.name
            ));
        }

        for expected in &case.expect_codes {
            if !codes.contains(&expected.as_str()) {
                failures.push(format!(
                    "[{}] expected diagnostic {expected:?}, got {codes:?}",
                    case.name
                ));
            }
        }
    }

    assert!(failures.is_empty(), "corpus expectations:\n{}", failures.join("\n"));
}

#[test]
fn repaired_cases_match_expected_output() {
    // Collected rather than fail-fast: when a normalization tier regresses it
    // usually breaks several cases at once, and seeing them together is what
    // distinguishes "this one expectation is wrong" from "the tier is off".
    let mut failures: Vec<String> = Vec::new();

    for case in corpus().repaired {
        // `expected` describes what `md clean` produces, so the comparison is
        // against the pipeline. Input that starts out unparseable needs the
        // repair that restores parseability to land before the
        // parse-equivalent tier can be proven at all.
        let actual = clean_to_fixed_point(&case.source);
        if actual != case.expected {
            failures.push(format!(
                "[{}]\n  expected: {:?}\n  actual:   {:?}",
                case.name, case.expected, actual
            ));
        }
    }

    assert!(failures.is_empty(), "repair mismatches:\n{}", failures.join("\n"));
}

/// A `repaired` case whose input parses must not have its value changed by
/// the repair. Cases whose input does *not* parse are exempt: restoring
/// parseability is by definition a value change (there was no value before).
#[test]
fn repaired_parseable_cases_preserve_their_value() {
    for case in corpus().repaired {
        let analysis = analyze_yaml(&case.source);
        if !analysis.is_parseable() {
            continue;
        }
        let before: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&case.source).expect("parseable input");
        let after: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&analysis.apply().source).expect("repaired output must parse");
        assert_eq!(before, after, "[{}] repair changed the value", case.name);
    }
}

// --- Universal invariants -------------------------------------------------

/// Construction never fails on invalid YAML, so every case must analyze
/// without panicking — including the deliberately unparseable ones.
#[test]
fn analysis_never_panics_across_corpus() {
    for (name, source) in all_cases() {
        let analysis = analyze_yaml(&source);
        let _ = analysis.diagnostics();
        let _ = analysis.repairs().count();
        let _ = analysis.apply();
        // Every case is either parseable or carries a projected failure;
        // neither accessor may be simultaneously empty.
        assert!(
            analysis.parsed_value().is_some() || analysis.parse_failure().is_some(),
            "[{name}] analysis must carry either a value or a failure"
        );
    }
}

#[test]
fn repair_is_deterministic_across_corpus() {
    for (name, source) in all_cases() {
        let first = analyze_yaml(&source).apply();
        let second = analyze_yaml(&source).apply();
        assert_eq!(first.source, second.source, "[{name}] repair is not stable");
    }
}

/// The spec's fixed-point requirement: cleaning cleaned output is a no-op.
/// Without this, `md clean` in a pre-commit hook would keep producing new
/// diffs on unchanged files.
///
/// The fixed point is a property of the *pipeline*, not of a single
/// `apply()`. Parse-equivalence-gated repairs cannot be proven while the
/// document is unparseable — there is no original `Value` to compare a
/// candidate against — so an unparseable input needs the reserved-indicator
/// repair to land before normalization becomes provable. `md clean` closes
/// that gap by rescanning once after an edit restores parseability
/// (`cli/src/commands/clean/frontmatter_repair.rs`), and this test asserts
/// against that same loop. See [`single_pass_is_a_fixed_point_once_parseable`]
/// for the narrower guarantee the single-call API does make.
#[test]
fn pipeline_is_idempotent_across_corpus() {
    for (name, source) in all_cases() {
        let once = clean_to_fixed_point(&source);
        let twice = clean_to_fixed_point(&once);
        assert_eq!(
            twice, once,
            "[{name}] second clean changed the output; repair is not a fixed point"
        );
    }
}

/// For input that already parses, one `apply()` is enough — every repair was
/// provable on the first pass. This is the guarantee a library consumer of
/// `analyze_yaml` can rely on without replicating the CLI's loop.
#[test]
fn single_pass_is_a_fixed_point_once_parseable() {
    for (name, source) in all_cases() {
        let analysis = analyze_yaml(&source);
        if !analysis.is_parseable() {
            continue;
        }
        let once = analysis.apply().source;
        let twice = analyze_yaml(&once).apply();
        assert_eq!(
            twice.source, once,
            "[{name}] parseable input must reach its fixed point in one pass"
        );
        assert!(
            !twice.changed(),
            "[{name}] second clean applied an edit to already-clean output"
        );
    }
}

#[test]
fn untouched_bytes_preserved_across_corpus() {
    for (name, source) in all_cases() {
        let outcome = analyze_yaml(&source).apply();
        assert_untouched_bytes_preserved(&name, &source, &outcome);
    }
}

/// Pins the parser quirk that the BOM recovery path exists to work around.
///
/// A byte-order mark reads as a document boundary, so a BOM followed by more
/// than one top-level key is rejected as a multi-document stream — while a
/// single-key block survives. That asymmetry is why the defect only ever
/// showed up on real frontmatter. If a future `serde_yaml_ng` fixes this,
/// this test fails and `bom_recovery` can be deleted.
#[test]
fn bom_makes_multi_key_documents_unparseable() {
    assert!(
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>("\u{FEFF}key: value\n").is_ok(),
        "a single-key BOM document is expected to parse"
    );
    assert!(
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>("\u{FEFF}alpha: 1\nbeta: 2\n").is_err(),
        "a multi-key BOM document is expected to be rejected as multi-document; \
         if this now parses, the bom_recovery branch is dead code"
    );
}

/// Regression: a BOM must never suppress the repairs that accompany it.
///
/// Before the `bom_recovery` branch existed, a BOM plus more than one key
/// made the block unparseable, which skipped candidate generation entirely —
/// so neither the mark nor the CRLF line endings were normalized, and
/// `md clean` failed the document instead of fixing it. This is the exact
/// shape of a Windows-authored frontmatter block.
#[test]
fn bom_does_not_suppress_line_ending_normalization() {
    let source = "\u{FEFF}alpha: 1\r\nbeta: 2\r\n";

    let first = analyze_yaml(source);
    assert!(
        !first.is_parseable(),
        "precondition: this shape does not parse as authored"
    );
    assert!(
        first
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "yaml.bom"),
        "the BOM must be reported even though the document does not parse"
    );

    let recovered = first.apply().source;
    assert_eq!(
        recovered, "alpha: 1\r\nbeta: 2\r\n",
        "the first pass removes the mark, which is what restores parseability"
    );

    assert_eq!(
        clean_to_fixed_point(source),
        "alpha: 1\nbeta: 2\n",
        "the pipeline must go on to normalize the line endings the BOM was hiding"
    );
}

/// Guards the corpus itself: an empty or silently-truncated corpus file would
/// make every test above vacuously pass.
#[test]
fn corpus_is_populated_and_uniquely_named() {
    let cases = all_cases();
    assert!(
        cases.len() >= 20,
        "corpus shrank to {} cases; it is meant to cover the feature's failure classes",
        cases.len()
    );

    let mut names: Vec<&str> = cases.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(total, names.len(), "corpus case names must be unique");
}
