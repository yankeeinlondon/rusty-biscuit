//! Source fidelity: terminators, byte-order marks, final-newline presence,
//! unused reference definitions, and authored whitespace survive cleanup;
//! inserted lines take their neighbor's terminator; nothing adds trailing
//! whitespace.

use super::body_support::{clean_accepted, only, profiles, rules_with};
use crate::markdown::cleanup::diagnostic::FallbackReason;
use crate::markdown::cleanup::rules::{Leaf, ResolvedMarkdownRules};
use crate::markdown::cleanup::{CleanOutcome, clean_body};

/// Documents exercising every rule, used for the corpus-style properties.
const CORPUS: [&str; 6] = [
    "# Title\n\nIntro:\n- a\n  - b\n\n```\ncode   \n```\n\n| a | bb |\n|---|---|\n| c | d |\n",
    "Text with trailing space \nand a hard break  \nhere.\n\n1. one\n3. three\n\n> quote\n> - x\n",
    "one\r\ntwo\nthree\r\n\r\n- a\n  - b\r\n",
    "\u{FEFF}- a\n  - b\n",
    "Intro:\n- a\n  - b",
    "[used] text\n\n[used]: /u\n[unused]: https://example.com \"Unused\"\n",
];

#[test]
fn every_body_rule_off_returns_every_document_byte_for_byte() {
    let rules = ResolvedMarkdownRules::all_body_rules_off();
    for source in CORPUS {
        let cleaned = clean_body(source, &rules);
        assert_eq!(cleaned.outcome, CleanOutcome::Unchanged, "{source:?}");
        assert_eq!(cleaned.text, source);
    }
}

#[test]
fn crlf_terminators_survive_and_inserted_lines_use_them() {
    let source = "Intro:\r\n- a\r\n  - b\r\n";
    assert_eq!(clean_accepted(source, &ResolvedMarkdownRules::defaults()), "Intro:\r\n\r\n- a\r\n    - b\r\n");
}

#[test]
fn mixed_terminators_are_kept_line_by_line() {
    let source = "one\r\ntwo\nthree\r\n\r\n- a\n  - b\r\n";
    assert_eq!(
        clean_accepted(source, &ResolvedMarkdownRules::defaults()),
        "one\r\ntwo\nthree\r\n\r\n- a\n    - b\r\n"
    );
    let unwrap = rules_with(|o| o.unwrap = Leaf::Value(true));
    assert_eq!(clean_accepted(source, &unwrap), "one two three\r\n\r\n- a\n    - b\r\n");
    // An inserted blank line takes the terminator of the line above it.
    assert_eq!(clean_accepted("Intro:\n- a\r\n", &ResolvedMarkdownRules::defaults()), "Intro:\n\n- a\r\n");
}

#[test]
fn a_lone_carriage_return_is_a_line_break() {
    let unwrap = only(|o| o.unwrap = Leaf::Value(true));
    assert_eq!(clean_accepted("a\rb\r\n", &unwrap), "a b\r\n");
    assert_eq!(clean_accepted("a\rb\r\n", &ResolvedMarkdownRules::all_body_rules_off()), "a\rb\r\n");
}

#[test]
fn a_byte_order_mark_is_kept_and_does_not_hide_the_first_block() {
    assert_eq!(
        clean_accepted("\u{FEFF}- a\n  - b\n", &ResolvedMarkdownRules::defaults()),
        "\u{FEFF}- a\n    - b\n"
    );
    let cleaned = clean_body("\u{FEFF}::end-block\n", &ResolvedMarkdownRules::defaults());
    assert_eq!(cleaned.outcome, CleanOutcome::Declined);
    assert_eq!(cleaned.text, "\u{FEFF}::end-block\n");
}

#[test]
fn final_newline_presence_is_kept() {
    let rules = ResolvedMarkdownRules::defaults();
    assert_eq!(clean_accepted("Intro:\n- a\n  - b", &rules), "Intro:\n\n- a\n    - b");
    assert_eq!(clean_accepted("text", &rules), "text");
    assert_eq!(clean_accepted("text\n\n\n", &rules), "text\n\n\n");
    let leading = "\n\n# Title\n";
    assert_eq!(clean_accepted(leading, &rules), leading);
}

#[test]
fn empty_and_blank_bodies_are_unchanged() {
    for source in ["", "\n", "   \n\n", "\r\n"] {
        let cleaned = clean_body(source, &ResolvedMarkdownRules::defaults());
        assert_eq!(cleaned.outcome, CleanOutcome::Unchanged, "{source:?}");
        assert_eq!(cleaned.text, source);
    }
}

#[test]
fn unused_reference_definitions_are_kept_under_every_rule() {
    let source = CORPUS[5];
    for (profile, rules) in profiles() {
        let cleaned = clean_accepted(source, &rules);
        assert!(cleaned.contains("[unused]: https://example.com \"Unused\"\n"), "{profile}: {cleaned:?}");
    }
}

#[test]
fn no_rule_introduces_trailing_whitespace() {
    for source in CORPUS {
        let authored: Vec<&str> = source.lines().filter(|line| line.ends_with([' ', '\t'])).collect();
        for (profile, rules) in profiles() {
            let cleaned = clean_accepted(source, &rules);
            for line in cleaned.lines().filter(|line| line.ends_with([' ', '\t'])) {
                // The last word and the whitespace after it must be authored
                // (a joined line may end in an authored hard break).
                let word_start = line.trim_end().rfind(' ').map_or(0, |index| index + 1);
                let ending = &line[word_start..];
                assert!(
                    authored.iter().any(|authored| authored.ends_with(ending)),
                    "{profile} introduced trailing whitespace on {line:?} in {cleaned:?}"
                );
            }
        }
    }
}

#[test]
fn every_profile_is_idempotent_on_the_corpus() {
    for source in CORPUS {
        for (_, rules) in profiles() {
            // `clean_accepted` asserts a second run is a no-op.
            clean_accepted(source, &rules);
        }
    }
}

#[test]
fn a_malformed_region_is_reported_at_its_line() {
    let source = "# Title\n\nText\n::end-block\n- a\n  - b\n";
    let cleaned = clean_body(source, &ResolvedMarkdownRules::defaults());
    assert_eq!(cleaned.outcome, CleanOutcome::Declined);
    assert_eq!(cleaned.text, source);
    let fallback = cleaned.fallback.unwrap();
    assert_eq!(fallback.reason, FallbackReason::MalformedRegion);
    assert_eq!(fallback.offset, source.find("::end-block").unwrap());
    assert!(cleaned.diagnostics[0].message.contains("line 4, column 1"), "{}", cleaned.diagnostics[0].message);
    assert_eq!(cleaned.diagnostics[0].fallback.as_ref().map(|fallback| fallback.reason), Some(FallbackReason::MalformedRegion));
}

#[test]
fn a_fallback_reanchors_into_a_containing_document() {
    let body = "Text\n::end-block\n";
    let document = format!("---\ntitle: x\n---\n{body}");
    let cleaned = clean_body(body, &ResolvedMarkdownRules::defaults());
    let diagnostic = cleaned.fallback.unwrap().diagnostic(&document, document.len() - body.len());
    assert_eq!(diagnostic.span, 0..document.len());
    assert!(diagnostic.message.contains("line 5, column 1"), "{}", diagnostic.message);
}
