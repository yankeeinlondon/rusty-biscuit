//! Helpers for the guarded body-cleanup suites.

use pulldown_cmark::{Options, Parser, html};

use crate::markdown::cleanup::rules::{
    DefaultLanguage, FixedWidth, Indentation, Leaf, ListInteriorPadding, ListItemPadding, MarkdownRuleOverrides,
    ResolvedMarkdownRules, resolve,
};
use crate::markdown::cleanup::{BodyCleanup, CleanOutcome, clean_body};

/// Rules with the given leaves set explicitly (as a CLI would) and every other
/// rule at its default.
pub(super) fn rules_with(set: impl FnOnce(&mut MarkdownRuleOverrides)) -> ResolvedMarkdownRules {
    let mut overrides = MarkdownRuleOverrides::default();
    set(&mut overrides);
    resolve(&overrides, None, None)
}

/// Every body rule off except the given leaves.
pub(super) fn only(set: impl FnOnce(&mut MarkdownRuleOverrides)) -> ResolvedMarkdownRules {
    let mut overrides = MarkdownRuleOverrides::all_body_rules_off();
    set(&mut overrides);
    resolve(&overrides, None, None)
}

/// Every non-default value of every body rule, plus the default and
/// all-off profiles, named for failure messages.
pub(super) fn profiles() -> Vec<(&'static str, ResolvedMarkdownRules)> {
    vec![
        ("defaults", ResolvedMarkdownRules::defaults()),
        ("all body rules off", ResolvedMarkdownRules::all_body_rules_off()),
        ("table-layout: false", rules_with(|o| o.table_layout = Leaf::Value(false))),
        ("unwrap: true", rules_with(|o| o.unwrap = Leaf::Value(true))),
        ("fixed-width: 80", rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(80)))),
        ("fixed-width: 20", rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(20)))),
        ("indentation: 2", rules_with(|o| o.indentation = Leaf::Value(Indentation::Two))),
        ("indentation: 8", rules_with(|o| o.indentation = Leaf::Value(Indentation::Eight))),
        ("indentation: preserve", rules_with(|o| o.indentation = Leaf::Value(Indentation::Preserve))),
        ("ordered-lists: false", rules_with(|o| o.ordered_lists = Leaf::Value(false))),
        ("list-padding: false", rules_with(|o| o.list_padding = Leaf::Value(false))),
        ("list-item-padding: none", rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::None))),
        ("list-item-padding: one", rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One))),
        (
            "list-interior-padding: none",
            rules_with(|o| o.list_interior_padding = Leaf::Value(ListInteriorPadding::None)),
        ),
        (
            "list-interior-padding: except-nested-lists",
            rules_with(|o| o.list_interior_padding = Leaf::Value(ListInteriorPadding::ExceptNestedLists)),
        ),
        (
            "list-interior-padding: all",
            rules_with(|o| o.list_interior_padding = Leaf::Value(ListInteriorPadding::All)),
        ),
        (
            "default-language: \"\"",
            rules_with(|o| o.default_language = Leaf::Value(DefaultLanguage::new("").unwrap())),
        ),
        (
            "default-language: sh",
            rules_with(|o| o.default_language = Leaf::Value(DefaultLanguage::new("sh").unwrap())),
        ),
    ]
}

/// Cleans `source`, asserting the guard accepted the result and that a
/// second run leaves it unchanged (idempotence).
#[track_caller]
pub(super) fn clean_accepted(source: &str, rules: &ResolvedMarkdownRules) -> String {
    let cleaned = clean_body(source, rules);
    assert_ne!(
        cleaned.outcome,
        CleanOutcome::Declined,
        "cleanup declined {source:?}: {:?}",
        cleaned.diagnostics
    );
    assert_idempotent(&cleaned, rules);
    cleaned.text
}

#[track_caller]
pub(super) fn assert_idempotent(cleaned: &BodyCleanup, rules: &ResolvedMarkdownRules) {
    let again = clean_body(&cleaned.text, rules);
    assert_eq!(again.text, cleaned.text, "cleanup is not idempotent");
    assert_eq!(again.outcome, CleanOutcome::Unchanged, "a second run changed {:?}", cleaned.text);
}

/// The CommonMark structure of `source` as rendered HTML with whitespace runs
/// collapsed (so a joined or re-wrapped soft break compares equal) and
/// fence language classes removed. `ignore_tightness` also drops paragraph
/// tags, for the padding rules allowed to change tight/loose. An oracle
/// independent of the cleanup's own comparison.
pub(super) fn structure(source: &str, ignore_tightness: bool) -> String {
    let options = Options::all() - Options::ENABLE_SMART_PUNCTUATION - Options::ENABLE_DEFINITION_LIST;
    let mut rendered = String::new();
    html::push_html(&mut rendered, Parser::new_ext(source, options));
    // An added language is an allowed change; exact info strings are asserted
    // by the text expectations.
    while let Some(start) = rendered.find(" class=\"language-") {
        let end = rendered[start + 1..].find("\">").map_or(rendered.len(), |end| start + 1 + end + 1);
        rendered.replace_range(start..end, "");
    }
    if ignore_tightness {
        rendered = rendered.replace("<p>", "").replace("</p>", "");
    }
    rendered.split_whitespace().collect::<Vec<_>>().join(" ").replace("> ", ">").replace(" <", "<")
}
