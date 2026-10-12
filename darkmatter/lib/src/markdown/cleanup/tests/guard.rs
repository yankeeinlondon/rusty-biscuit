//! The fidelity guard: a candidate that changes meaning is refused and the
//! input returned with one `clean.fidelity-fallback` finding. The broken-pass
//! hook corrupts the candidate after every rule has run, so each test proves
//! the final comparison, not a rule, caught the corruption.

use biscuit_file::YamlCertainty;

use super::body_support::rules_with;
use crate::markdown::cleanup::diagnostic::{CleanCode, CleanStage, FallbackReason};
use crate::markdown::cleanup::rules::{
    FixedWidth, Leaf, ListItemPadding, ResolvedMarkdownRules,
};
use crate::markdown::cleanup::test_hooks::{BrokenPass, install_broken_pass};
use crate::markdown::cleanup::{BodyCleanup, CleanOutcome, clean_body};

const DOCUMENT: &str = concat!(
    "Link [a](https://a.example \"T\") and *em* and <b>bold</b>.\n",
    "Hard  \n",
    "break and {{ name }} here.\n",
    "\n",
    "- [ ] task\n",
    "- b\n",
    "\n",
    "| x | y |\n",
    "|:-:|---|\n",
    "| 1 | 2 |\n",
    "\n",
    "[ref]: /one\n",
    "[unused]: /unused \"Title\"\n",
    "\n",
    "```rust\n",
    "code line\n",
    "```\n",
);

#[track_caller]
fn assert_refused(cleaned: &BodyCleanup, source: &str, location: &str) {
    assert_eq!(cleaned.outcome, CleanOutcome::Declined, "{:?}", cleaned.text);
    assert_eq!(cleaned.text, source, "a refusal returns the input byte for byte");
    assert_eq!(cleaned.diagnostics.len(), 1);
    let diagnostic = &cleaned.diagnostics[0];
    assert_eq!(diagnostic.code, CleanCode::FidelityFallback);
    assert_eq!(diagnostic.stage, CleanStage::Fidelity);
    assert_eq!(diagnostic.classification, YamlCertainty::NonDeterministicFind);
    assert!(diagnostic.repairs.is_empty());
    assert_eq!(diagnostic.fallback.as_ref().map(|fallback| fallback.reason), Some(FallbackReason::SemanticMismatch));
    assert_eq!(diagnostic.span, 0..source.len(), "the finding spans the whole text");
    assert!(diagnostic.message.contains(location), "{location} in {:?}", diagnostic.message);
}

/// Runs `source` under `rules` with `pass` corrupting the candidate.
fn with_broken_pass(source: &str, rules: &ResolvedMarkdownRules, pass: BrokenPass) -> BodyCleanup {
    let _guard = install_broken_pass(pass);
    clean_body(source, rules)
}

#[test]
fn an_identity_pass_is_accepted() {
    let cleaned = with_broken_pass(DOCUMENT, &ResolvedMarkdownRules::defaults(), str::to_string);
    assert_eq!(cleaned.outcome, CleanOutcome::Changed, "{:?}", cleaned.diagnostics);
}

#[test]
fn every_meaning_change_is_refused_at_its_first_input_block() {
    let defaults = ResolvedMarkdownRules::defaults();
    let cases: [(&str, BrokenPass, &str); 13] = [
        ("link target", |c| c.replacen("https://a.example", "https://b.example", 1), "line 1, column 6"),
        ("link title", |c| c.replacen("\"T\"", "\"U\"", 1), "line 1, column 6"),
        ("emphasis", |c| c.replacen("*em*", "em", 1), "line 1, column 32"),
        ("hard break", |c| c.replacen("Hard  \n", "Hard\n", 1), "line 2, column 5"),
        ("inline HTML", |c| c.replacen("<b>", "<i>", 1), "line 1, column 46"),
        ("task state", |c| c.replacen("[ ]", "[x]", 1), "line 5, column 3"),
        ("table alignment", |c| c.replacen(":-:", "---", 1), "line 8, column 1"),
        ("reference definition", |c| c.replacen("/one", "/two", 1), "line 12, column 1"),
        ("unused reference definition", |c| c.replacen("\"Title\"", "\"Other\"", 1), "line 13, column 1"),
        ("expression", |c| c.replacen("{{ name }}", "{{name}}", 1), "line 3, column 1"),
        ("code content", |c| c.replacen("code line", "code  line", 1), "line 15, column 1"),
        ("info string", |c| c.replacen("```rust", "```rusty", 1), "line 15, column 1"),
        ("joined soft break", |c| c.replacen(".\nHard", ". Hard", 1), "line 1, column 57"),
    ];
    for (name, pass, location) in cases {
        let cleaned = with_broken_pass(DOCUMENT, &defaults, pass);
        assert_eq!(cleaned.outcome, CleanOutcome::Declined, "{name} was not refused");
        assert_refused(&cleaned, DOCUMENT, location);
    }
}

#[test]
fn code_block_terminators_are_compared_from_source() {
    let source = "```text\r\nline\r\n```\r\n";
    let cleaned = with_broken_pass(source, &ResolvedMarkdownRules::defaults(), |c| c.replacen("line\r\n", "line\n", 1));
    assert_refused(&cleaned, source, "line 1, column 1");
}

#[test]
fn a_blank_line_next_to_a_directive_is_refused() {
    let source = "Para\n::file ./x.md\nMore\n";
    let cleaned = with_broken_pass(source, &ResolvedMarkdownRules::defaults(), |c| c.replacen("::file", "\n::file", 1));
    // The soft break that crossed the directive line is the first change.
    assert_refused(&cleaned, source, "line 1, column 5");
}

#[test]
fn a_directive_left_behind_by_a_moved_list_is_refused() {
    let source = "- a\n  - b\n    ::file ./x.md\n";
    let rules = ResolvedMarkdownRules::all_body_rules_off();
    let cleaned = with_broken_pass(source, &rules, |c| c.replacen("  - b", "    - b", 1));
    assert_refused(&cleaned, source, "line 3, column 1");
}

#[test]
fn default_mode_refuses_a_tight_list_becoming_loose() {
    let source = "- a\n- b\n";
    let cleaned = with_broken_pass(source, &ResolvedMarkdownRules::defaults(), |c| c.replacen("- a\n", "- a\n\n", 1));
    assert_refused(&cleaned, source, "line 1, column 3");
}

#[test]
fn an_explicit_padding_rule_permits_only_the_lists_it_reaches() {
    let one = rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One));
    // A list with siblings may change spacing under list-item-padding...
    let siblings = "- a\n- b\n";
    let cleaned = with_broken_pass(siblings, &one, |c| c.replacen("- a\n\n- b", "- a\n- b", 1).replacen("- a\n- b", "- a\n\n- b", 1));
    assert_eq!(cleaned.outcome, CleanOutcome::Changed, "{:?}", cleaned.diagnostics);
    // ...but a single-item list's interior belongs to list-interior-padding,
    // which was not selected.
    let single = "- a\n    - x\n";
    let cleaned = with_broken_pass(single, &one, |c| c.replacen("- a\n", "- a\n\n", 1));
    assert_refused(&cleaned, single, "line 1, column 3");
}

#[test]
fn ordered_numbers_change_only_by_sequential_renumbering_under_the_rule() {
    let source = "1. a\n2. b\n";
    let off = rules_with(|o| o.ordered_lists = Leaf::Value(false));
    let cleaned = with_broken_pass(source, &off, |c| c.replacen("2. b", "3. b", 1));
    assert_refused(&cleaned, source, "line 1, column 1");
    let cleaned = with_broken_pass(source, &ResolvedMarkdownRules::defaults(), |c| c.replacen("2. b", "5. b", 1));
    assert_refused(&cleaned, source, "line 1, column 1");
}

#[test]
fn a_failure_introduced_after_reflow_is_refused() {
    let source = "one two three four five six seven\n";
    let rules = rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(20)));
    // The re-wrap stage produced "one two three four\nfive six seven"; the
    // broken pass turns the continuation into a list item.
    let cleaned = with_broken_pass(source, &rules, |c| c.replacen("\nfive", "\n- five", 1));
    assert_refused(&cleaned, source, "line 1, column 1");
}

#[test]
fn a_corrupted_unchanged_input_is_still_checked() {
    let source = "Nothing to format.\n";
    let cleaned = with_broken_pass(source, &ResolvedMarkdownRules::defaults(), |c| c.replace("Nothing", "Something"));
    assert_refused(&cleaned, source, "line 1, column 1");
}

#[test]
fn an_output_only_block_is_located_at_the_end_of_the_input() {
    let source = "Para\n";
    let cleaned = with_broken_pass(source, &ResolvedMarkdownRules::defaults(), |c| format!("{c}\n# Added\n"));
    assert_refused(&cleaned, source, "line 2, column 1");
}
