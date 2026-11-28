//! Where a failing `match` condition is reported. A condition that fails its
//! type grammar, uses a constraint a match may not, or mixes combinators
//! with property keys is located at its own node, however deep in nested
//! `all`/`group` lists it sits, through every route that reports it: the
//! trigger parser's [`trigger_error_span`], whole-YAML kind checks, and
//! schema-root trigger discovery. The fixture shares the failing property's
//! name with its payload, so a route that loses the condition's path cannot
//! land on the right node by accident.

use simplified_schema::triggers::{parse_trigger_envelope_from_str, trigger_error_span};
use simplified_schema::yaml_documents::load_yaml_document;

/// A trigger shaped like Claudine's inline-composition trigger: its
/// conditions sit in a `group` under `all`, and its payload declares the same
/// `prompt` property.
const GROUPED: &str = "\
kind: schema-trigger
match:
    all:
        - group:
              - kind: literal(inline-compose; required)
              - prompt: string(required)
        - none:
              - sequence: string(required)
$schema:
    prompt: string
";

/// The flat form of the same trigger.
const FLAT: &str = "\
kind: schema-trigger
match:
    prompt: string(required)
$schema:
    prompt: string
";

fn edit(base: &str, from: &str, to: &str) -> String {
    assert!(base.contains(from), "`{from}` is not in {base:?}");
    base.replacen(from, to, 1)
}

/// The 1-based line and column of byte `offset` in `text`.
fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    (before.matches('\n').count() + 1, before[before.rfind('\n').map_or(0, |at| at + 1)..].chars().count() + 1)
}

/// The position the parser's error for `text` is located at.
fn parser_position(text: &str) -> Option<(usize, usize)> {
    let error = parse_trigger_envelope_from_str(text).expect_err("the trigger is refused");
    trigger_error_span(text, &error).map(|span| line_column(text, span.start))
}

/// Each failing condition and the authored token its error must point at.
fn failing_conditions() -> Vec<(&'static str, String, &'static str)> {
    vec![
        ("flat typo", edit(FLAT, "string(required)", "string(requird)"), "string(requird)"),
        (
            "grouped literal typo",
            edit(GROUPED, "literal(inline-compose; required)", "literal(inline-compose; requird)"),
            "literal(inline-compose; requird)",
        ),
        ("grouped typo", edit(GROUPED, "prompt: string(required)", "prompt: string(requird)"), "string(requird)"),
        (
            "flat forbidden import",
            edit(FLAT, "string(required)", "\"prompt(required)@./document.yaml\""),
            "\"prompt(required)@",
        ),
        (
            "grouped forbidden import",
            edit(GROUPED, "prompt: string(required)", "prompt: \"prompt(required)@./document.yaml\""),
            "\"prompt(required)@",
        ),
        (
            "grouped mixed mapping",
            edit(GROUPED, "- prompt: string(required)", "- prompt: string(required)\n                $path: \"*.md\""),
            "prompt: string(required)\n",
        ),
    ]
}

/// A trigger that omits its required `match`, after a comment header, so the
/// document's mapping does not start at its first line.
const MISSING_MATCH: &str = "\
# A trigger without its matcher.

kind: schema-trigger
name: no-matcher
$schema:
    prompt: string
";

/// Where a refusal of [`MISSING_MATCH`] points: the mapping that lacks
/// `match`, at its first key.
fn missing_match_position() -> (usize, usize) {
    line_column(MISSING_MATCH, MISSING_MATCH.find("kind:").unwrap())
}

#[test]
fn an_omitted_match_is_located_at_the_mapping_that_lacks_it() {
    assert_eq!(parser_position(MISSING_MATCH), Some(missing_match_position()));
    assert_eq!(missing_match_position(), (3, 1));
}

/// The whole-YAML check of the same file points at the same mapping.
#[test]
fn whole_yaml_checks_locate_an_omitted_match_at_the_same_mapping() {
    let tmp = tempfile::tempdir().unwrap();
    let problems = crate::envelope_null_carriers::whole_yaml_problems(MISSING_MATCH, tmp.path());
    let [problem] = problems.as_slice() else { panic!("one problem: {problems:#?}") };
    let loaded = load_yaml_document(MISSING_MATCH, &tmp.path().join("subject.yaml")).unwrap();
    let mapped = loaded.byte_range(problem).map(|range| line_column(MISSING_MATCH, range.start));
    assert_eq!(mapped, Some(missing_match_position()), "{problem:#?}");
}

#[test]
fn a_failing_condition_is_located_at_its_own_node() {
    for (case, text, needle) in failing_conditions() {
        let expected = line_column(&text, text.find(needle).unwrap());
        assert_eq!(parser_position(&text), Some(expected), "{case}");
    }
}

/// The controls keep their positions: an invalid list element at its index,
/// an empty, null, or scalar `match` at the `match` value, a bad label at the
/// label; the unedited triggers load.
#[test]
fn match_shape_and_label_controls_keep_their_positions() {
    for (case, text, needle) in [
        ("list element", "kind: schema-trigger\nmatch: [7]\n", "7]"),
        ("mixed list", "kind: schema-trigger\nmatch:\n  - prompt: string(required)\n  - 7\n", "7\n"),
        ("empty mapping", "kind: schema-trigger\nmatch: {}\n", "{}"),
        ("null", "kind: schema-trigger\nmatch: null\n", "null"),
        ("scalar", "kind: schema-trigger\nmatch: prompt\n", "prompt"),
        ("bad label", "kind: schema-trigger\nname: Bad_Name\nmatch:\n  prompt: string(required)\n", "Bad_Name"),
    ] {
        let expected = line_column(text, text.find(needle).unwrap());
        assert_eq!(parser_position(text), Some(expected), "{case}");
    }
    for valid in [FLAT, GROUPED] {
        assert!(parse_trigger_envelope_from_str(valid).expect("loads").is_some());
    }
}

/// A trigger file checked as a whole YAML document reports grammar typos at
/// the definition's exact paths and a forbidden condition at its own path,
/// flat or grouped.
#[test]
fn whole_yaml_checks_locate_a_condition_at_its_nested_path() {
    let tmp = tempfile::tempdir().unwrap();
    for (case, text, pointer, needle) in [
        ("flat typo", edit(FLAT, "string(required)", "string(requird)"), "/match/prompt", "string(requird)"),
        (
            "grouped literal typo",
            edit(GROUPED, "literal(inline-compose; required)", "literal(inline-compose; requird)"),
            "/match/all/0/group/0/kind",
            "literal(inline-compose; requird)",
        ),
        (
            "flat forbidden import",
            edit(FLAT, "string(required)", "\"prompt(required)@./document.yaml\""),
            "/match/prompt",
            "\"prompt(required)@",
        ),
        (
            "grouped forbidden import",
            edit(GROUPED, "prompt: string(required)", "prompt: \"prompt(required)@./document.yaml\""),
            "/match/all/0/group/1/prompt",
            "\"prompt(required)@",
        ),
    ] {
        let problems = crate::envelope_null_carriers::whole_yaml_problems(&text, tmp.path());
        let [problem] = problems.as_slice() else { panic!("{case}: one problem: {problems:#?}") };
        assert_eq!(problem.path, pointer, "{case}: {problem:#?}");
        // A definition failure is positioned by the host's map, a kind check
        // by the check itself; both must name the condition's own value.
        let expected = line_column(&text, text.find(needle).unwrap());
        let loaded = load_yaml_document(&text, &tmp.path().join("subject.yaml")).unwrap();
        let mapped = loaded.byte_range(problem).map(|range| line_column(&text, range.start));
        assert_eq!(mapped, Some(expected), "{case}: {problem:#?}");
        if let (Some(line), Some(column)) = (problem.line, problem.column) {
            assert_eq!((line as usize, column as usize), expected, "{case}: {problem:#?}");
        }
    }
}

#[cfg(feature = "file")]
mod discovery {
    use std::collections::HashMap;
    use std::fs;

    use biscuit_file::{FileResolutionContext, canonicalize_simplified};
    use simplified_schema::embedded::ShippedSources;
    use simplified_schema::triggers::scan;

    use super::{MISSING_MATCH, failing_conditions, line_column, missing_match_position};

    /// Schema-root discovery for a document skips the broken trigger with a
    /// warning located at the failing condition.
    #[test]
    fn a_skipped_trigger_is_located_at_its_failing_condition() {
        for (case, text, needle) in failing_conditions() {
            let tmp = tempfile::tempdir().unwrap();
            let root = canonicalize_simplified(tmp.path()).unwrap();
            fs::create_dir_all(root.join("schemas")).unwrap();
            fs::write(root.join("schemas").join("trigger.yaml"), &text).unwrap();
            let context = FileResolutionContext::from_snapshot(root.clone(), None, HashMap::new()).with_package_root(&root);

            let registry = scan(&context, ShippedSources::NONE).expect("discovery completes");

            assert!(registry.triggers.is_empty(), "{case}: the trigger is skipped");
            let [finding] = registry.findings() else { panic!("{case}: one finding: {:#?}", registry.findings()) };
            let expected = line_column(&text, text.find(needle).unwrap());
            assert_eq!(finding.position(), Some(expected), "{case}: {finding:#?}");
        }
    }

    /// A trigger without `match` found in an ordinary schema root is skipped
    /// with a warning at the mapping that lacks it.
    #[test]
    fn a_skipped_trigger_without_match_is_located_at_its_mapping() {
        let tmp = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(tmp.path()).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(root.join("schemas").join("trigger.yaml"), MISSING_MATCH).unwrap();
        let context = FileResolutionContext::from_snapshot(root.clone(), None, HashMap::new()).with_package_root(&root);

        let registry = scan(&context, ShippedSources::NONE).expect("discovery completes");

        assert!(registry.triggers.is_empty(), "the trigger is skipped");
        let [finding] = registry.findings() else { panic!("one finding: {:#?}", registry.findings()) };
        assert_eq!(finding.position(), Some(missing_match_position()), "{finding:#?}");
    }
}
