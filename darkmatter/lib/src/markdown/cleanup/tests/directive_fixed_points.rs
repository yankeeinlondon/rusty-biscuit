//! Directive lines are fixed points under every rule: same bytes, same
//! terminator, same container, and no blank line gained or lost next to them.

use super::body_support::{clean_accepted, profiles, structure};
use crate::markdown::cleanup::rules::ResolvedMarkdownRules;
use crate::markdown::cleanup::{CleanOutcome, clean_body};

/// Each context ends a block with a directive line directly after it, and
/// surrounds it with content every rule wants to change: a soft break to
/// join, a long line to wrap, 2-space nesting, misnumbered items, an
/// unpadded list, an unaligned table, and an unlabeled fence.
const CONTEXTS: [(&str, &str); 4] = [
    (
        "after a block-quote paragraph",
        "Intro:\n- a\n- b\n\n> first quoted line that is long enough to be wrapped somewhere\n> second line\n::file ./quoted.md\n\n```\ncode\n```\n",
    ),
    (
        "after an ordered list paragraph nested two levels",
        "1. top\n   1. middle\n      1. deep paragraph line\n         joined line\n::block when=\"x\"\n\n3. after\n8. tail\n::end-block\n",
    ),
    (
        "after an unordered list paragraph nested two levels",
        "Lead:\n- top\n  - middle\n    - deep paragraph line\n      joined line\n::shell echo hi\n- next\n\n| a | bb |\n|---|---|\n| c | d |\n",
    ),
    (
        "after a table",
        "| a | bb |\n|---|:-:|\n| ccc | d |\n::file ./table-follow.md\n\nText one\ntext two.\n\n- x\n  - y\n",
    ),
];

/// Each protected line with whether the lines just above and below it are blank.
fn directive_neighborhoods(text: &str) -> Vec<(String, bool, bool)> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("::"))
        .map(|(index, line)| {
            let blank = |line: Option<&&str>| line.is_some_and(|line| line.trim().is_empty());
            ((*line).to_string(), index > 0 && blank(lines.get(index - 1)), blank(lines.get(index + 1)))
        })
        .collect()
}

#[test]
fn directive_lines_are_unchanged_under_every_rule_in_every_context() {
    for (context, source) in CONTEXTS {
        let expected = directive_neighborhoods(source);
        assert!(!expected.is_empty(), "{context}: fixture has a directive line");
        for (profile, rules) in profiles() {
            let cleaned = clean_accepted(source, &rules);
            assert_eq!(
                directive_neighborhoods(&cleaned),
                expected,
                "{context} under {profile}: directive line or its blank neighbors changed:\n{cleaned}"
            );
        }
    }
}

#[test]
fn the_contexts_are_really_formatted_by_the_defaults() {
    // The fixed-point test above means something only if the rules ran.
    let rules = ResolvedMarkdownRules::defaults();
    for (context, source) in CONTEXTS {
        assert_ne!(clean_accepted(source, &rules), source, "{context}");
    }
}

#[test]
fn a_directive_after_a_paragraph_is_never_joined_or_prefixed() {
    let source = "> quoted paragraph\n::file ./x.md\n> more\n";
    for (profile, rules) in profiles() {
        assert_eq!(clean_accepted(source, &rules), source, "{profile}");
    }
}

#[test]
fn ordinary_markdown_between_block_directives_is_formatted() {
    let source = "::block when=\"a\"\nIntro:\n- a\n  - b\n```\nx\n```\n::end-block\n";
    assert_eq!(
        clean_accepted(source, &ResolvedMarkdownRules::defaults()),
        "::block when=\"a\"\nIntro:\n\n- a\n    - b\n\n```text\nx\n```\n::end-block\n"
    );
}

#[test]
fn quoted_shell_blocks_and_their_bodies_are_opaque() {
    let source = "> intro line\n> ::shell-block\n> - not a list\n>   1. nor this\n> ::end-block\n> after\n\n- a\n  - b\n";
    let cleaned = clean_accepted(source, &ResolvedMarkdownRules::defaults());
    assert!(cleaned.starts_with("> intro line\n> ::shell-block\n> - not a list\n>   1. nor this\n> ::end-block\n> after\n"));
    assert!(cleaned.ends_with("- a\n    - b\n"));
}

#[test]
fn quoted_shell_lines_are_protected() {
    let source = "> run this\n> ::shell echo hi\n> then\n";
    for (profile, rules) in profiles() {
        assert_eq!(clean_accepted(source, &rules), source, "{profile}");
    }
}

#[test]
fn disclosure_lines_are_protected_and_their_body_is_formatted() {
    let source = "::disclosure Summary\n::details\n- a\n  - b\n::end-disclosure\n";
    assert_eq!(
        clean_accepted(source, &ResolvedMarkdownRules::defaults()),
        "::disclosure Summary\n::details\n- a\n    - b\n::end-disclosure\n"
    );
}

#[test]
fn directive_looking_code_is_formatted_as_code() {
    let source = "```\n::block when=\"x\"\n```\n";
    assert_eq!(
        clean_accepted(source, &ResolvedMarkdownRules::defaults()),
        "```text\n::block when=\"x\"\n```\n"
    );
}

#[test]
fn a_directive_keeps_its_crlf_terminator_and_column() {
    let source = "- a\r\n  - b\r\n    ::file ./nested.md\r\n";
    let cleaned = clean_body(source, &ResolvedMarkdownRules::defaults());
    // Moving `- b` would move the directive out of its item, so the list stays.
    assert_eq!(cleaned.outcome, CleanOutcome::Unchanged);
    assert_eq!(cleaned.text, source);
    assert_eq!(structure(&cleaned.text, false), structure(source, false));
}

#[test]
fn unpaired_block_directives_are_left_unchanged_and_reported() {
    for source in [
        "::block when=\"a\"\n- a\n  - b\n",
        "- a\n  - b\n::end-block\n",
        "::disclosure Open\n- a\n  - b\n",
    ] {
        let cleaned = clean_body(source, &ResolvedMarkdownRules::defaults());
        assert_eq!(cleaned.outcome, CleanOutcome::Declined, "{source:?}");
        assert_eq!(cleaned.text, source);
        let fallback = cleaned.fallback.expect("a declined result says why");
        assert_eq!(fallback.reason, crate::markdown::cleanup::diagnostic::FallbackReason::MalformedRegion);
    }
}
