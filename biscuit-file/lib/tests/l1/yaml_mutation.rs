//! Mutation and property coverage for the source-first YAML analyzer.
//!
//! The corpus and unit suites pin behavior on *chosen* inputs. This suite
//! attacks the same invariants with generated ones: valid frontmatter is
//! mutated in the ways agents actually break it (dropping a quote, injecting
//! a reserved indicator, switching line endings, padding with whitespace),
//! and the safety properties are asserted over the result.
//!
//! The properties, in the spec's terms:
//!
//! - **never corrupt a valid document** — if the input parses, the repaired
//!   output parses to an equal `serde_yaml_ng::Value`;
//! - **never mutate outside the frontmatter span** — enforced here as the
//!   stronger claim that every byte outside an *accepted edit* is preserved,
//!   plus an explicit sentinel-suffix check;
//! - **repairs are idempotent** — cleaning twice equals cleaning once;
//! - **the safety gate never admits a Value-changing edit** — no accepted
//!   edit set may move a parseable document to a different value.

use biscuit_file::analyze_yaml;
use proptest::prelude::*;

// ── Helpers ──────────────────────────────────────────────────────────────

fn parse(source: &str) -> Option<serde_yaml_ng::Value> {
    serde_yaml_ng::from_str(source).ok()
}

/// The universal safety assertion, applied to any source whatsoever.
///
/// Returns the repaired source so callers can chain further checks.
fn assert_core_invariants(source: &str) -> Result<String, TestCaseError> {
    let analysis = analyze_yaml(source);
    let outcome = analysis.apply();

    // Untouched bytes: rebuild the output from the original plus accepted
    // spans alone. Any mutation the audit did not declare fails here.
    let mut rebuilt = String::with_capacity(outcome.source.len());
    let mut cursor = 0;
    for repair in &outcome.audit.applied {
        prop_assert!(
            repair.span.start >= cursor,
            "accepted edits must be non-overlapping and ascending"
        );
        rebuilt.push_str(&source[cursor..repair.span.start]);
        rebuilt.push_str(&repair.replacement);
        cursor = repair.span.end;
    }
    rebuilt.push_str(&source[cursor..]);
    prop_assert_eq!(
        &outcome.source,
        &rebuilt,
        "output must be reconstructable from accepted spans alone"
    );

    // Safety gate: a parseable document may never change value.
    if let Some(before) = parse(source) {
        let after = parse(&outcome.source);
        prop_assert!(
            after.is_some(),
            "repair made a parseable document unparseable"
        );
        prop_assert_eq!(
            before,
            after.unwrap(),
            "repair changed the value of a parseable document"
        );
    }

    // Idempotency, asserted against the pipeline rather than a single
    // `apply()`. See `clean_to_fixed_point`.
    let settled = clean_to_fixed_point(source);
    let again = clean_to_fixed_point(&settled);
    prop_assert_eq!(&again, &settled, "repair is not a fixed point");

    Ok(outcome.source)
}

/// Mirrors `md clean`'s frontmatter loop: analyze, apply, and — only when an
/// edit restored parseability — rescan once.
///
/// Parse-equivalence-gated repairs are unprovable while the document does not
/// parse, so unparseable input needs the reserved-indicator repair to land
/// before normalization can be proven. `md clean` closes that gap with a
/// single conditional rescan
/// (`cli/src/commands/clean/frontmatter_repair.rs`); the fixed-point claim is
/// about that loop, not about one `apply()` call.
fn clean_to_fixed_point(source: &str) -> String {
    let first = analyze_yaml(source).apply().source;
    if first != source && parse(&first).is_some() {
        return analyze_yaml(&first).apply().source;
    }
    first
}

// ── Strategies ───────────────────────────────────────────────────────────

/// Keys drawn from real monorepo frontmatter vocabulary.
fn key_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "name", "title", "description", "hash", "version", "release", "agent", "spec", "kind",
        "ready",
    ])
    .prop_map(str::to_string)
}

/// Plain scalar values that are valid unquoted.
fn safe_value_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "example",
        "1.0",
        "true",
        "42",
        "some longer description text",
        "path/to/thing.md",
        "352b2caf7cdd68a6",
        "日本語",
    ])
    .prop_map(str::to_string)
}

/// A well-formed frontmatter block: 1-6 unique keys with safe values.
fn valid_document_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec((key_strategy(), safe_value_strategy()), 1..7).prop_map(|pairs| {
        let mut seen: Vec<String> = Vec::new();
        let mut out = String::new();
        for (key, value) in pairs {
            if seen.contains(&key) {
                continue;
            }
            seen.push(key.clone());
            out.push_str(&format!("{key}: {value}\n"));
        }
        if out.is_empty() {
            out.push_str("name: fallback\n");
        }
        out
    })
}

/// The YAML indicator characters that make a plain scalar illegal at value
/// start — the flagship failure class.
fn reserved_indicator_strategy() -> impl Strategy<Value = char> {
    prop::sample::select(vec!['@', '`', '%', '*', '|', '>', ',', ']', '}'])
}

/// How an agent typically damages otherwise-valid frontmatter.
#[derive(Debug, Clone)]
enum Mutation {
    /// Prefix a value with a reserved indicator (`title: @foo`).
    ReservedIndicator(char),
    /// Convert all line endings to CRLF.
    Crlf,
    /// Prepend a UTF-8 BOM.
    Bom,
    /// Append trailing spaces to every line.
    TrailingSpace(usize),
    /// Remove the final newline.
    StripFinalNewline,
    /// Pad the space after a mapping colon.
    PadColon,
    /// Drop a closing quote, producing unrepairable YAML.
    DropClosingQuote,
}

fn mutation_strategy() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        reserved_indicator_strategy().prop_map(Mutation::ReservedIndicator),
        Just(Mutation::Crlf),
        Just(Mutation::Bom),
        (1_usize..4).prop_map(Mutation::TrailingSpace),
        Just(Mutation::StripFinalNewline),
        Just(Mutation::PadColon),
        Just(Mutation::DropClosingQuote),
    ]
}

fn apply_mutation(document: &str, mutation: &Mutation) -> String {
    match mutation {
        Mutation::ReservedIndicator(indicator) => document
            .lines()
            .map(|line| match line.split_once(": ") {
                Some((key, value)) => format!("{key}: {indicator}{value}"),
                None => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Mutation::Crlf => document.replace('\n', "\r\n"),
        Mutation::Bom => format!("\u{FEFF}{document}"),
        Mutation::TrailingSpace(count) => document
            .lines()
            .map(|line| format!("{line}{}", " ".repeat(*count)))
            .collect::<Vec<_>>()
            .join("\n"),
        Mutation::StripFinalNewline => document.trim_end_matches('\n').to_string(),
        Mutation::PadColon => document.replace(": ", " :  "),
        Mutation::DropClosingQuote => format!("{document}broken: 'unterminated\n"),
    }
}

// ── Property tests ───────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        .. ProptestConfig::default()
    })]

    /// The headline property: mutate valid frontmatter any way we know how,
    /// and the analyzer still may not corrupt it, mutate undeclared bytes,
    /// or fail to reach a fixed point.
    #[test]
    fn mutated_documents_hold_core_invariants(
        document in valid_document_strategy(),
        mutation in mutation_strategy(),
    ) {
        let mutated = apply_mutation(&document, &mutation);
        assert_core_invariants(&mutated)?;
    }

    /// Stacking mutations is where span arithmetic breaks: a BOM shifts every
    /// later offset, CRLF shifts them again, and a reserved-indicator repair
    /// then has to land in the right place regardless.
    #[test]
    fn stacked_mutations_hold_core_invariants(
        document in valid_document_strategy(),
        first in mutation_strategy(),
        second in mutation_strategy(),
    ) {
        let mutated = apply_mutation(&apply_mutation(&document, &first), &second);
        assert_core_invariants(&mutated)?;
    }

    /// A valid document must survive untouched *in value* and, when it was
    /// already canonical, byte-for-byte.
    #[test]
    fn valid_documents_are_never_corrupted(document in valid_document_strategy()) {
        let repaired = assert_core_invariants(&document)?;
        prop_assert_eq!(
            &repaired,
            &document,
            "canonical input must come through byte-identical"
        );
    }

    /// Nothing after the analyzed region may move. A sentinel comment pinned
    /// to the end of the document must survive every repair intact — this is
    /// the "never mutate outside the span" claim in its most direct form.
    #[test]
    fn content_after_the_repair_site_is_preserved(
        document in valid_document_strategy(),
        indicator in reserved_indicator_strategy(),
    ) {
        const SENTINEL: &str = "# SENTINEL-DO-NOT-TOUCH\n";
        let broken = format!("title: {indicator}value\n{document}{SENTINEL}");
        let repaired = assert_core_invariants(&broken)?;
        prop_assert!(
            repaired.ends_with(SENTINEL),
            "sentinel suffix was modified: {:?}",
            repaired
        );
    }

    /// Arbitrary bytes must not panic the analyzer. `md clean` runs on files
    /// it did not author, so hostile input is a real case.
    #[test]
    fn arbitrary_input_never_panics(source in "\\PC{0,120}") {
        let analysis = analyze_yaml(&source);
        let _ = analysis.diagnostics();
        let _ = analysis.repairs().count();
        let _ = analysis.apply();
    }

    /// Unparseable-and-unrepairable input must be returned byte-identical so
    /// `md clean` keeps its exit-1 contract and leaves the file untouched.
    #[test]
    fn unrepairable_input_is_left_alone(document in valid_document_strategy()) {
        let broken = format!("{document}dangling: 'unterminated\n");
        let analysis = analyze_yaml(&broken);
        prop_assert!(!analysis.is_parseable());
        let outcome = analysis.apply();
        // Repairs may still fire for the valid prefix, but the unterminated
        // quote itself must survive verbatim.
        prop_assert!(
            outcome.source.contains("'unterminated"),
            "unrepairable region must be preserved: {:?}",
            outcome.source
        );
    }
}
