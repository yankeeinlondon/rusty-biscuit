use super::*;
use super::rules::{Indentation, Leaf, ListItemPadding, MarkdownRuleOverrides, resolve};

mod blockquote;
mod brackets;
mod emphasis;
mod escapes;
mod lists;
mod parse_count;
mod reflow;
mod tables;

/// Cleans `content` with `set` applied over the runtime defaults (at CLI
/// precedence), asserting the guard accepted the result and that a second run
/// is a fixed point. Prefer this over [`cleanup_content_with`] in tests: that
/// wrapper returns the input on refusal, so "output equals input" would also
/// pass for a decline.
#[track_caller]
fn clean_with(content: &str, set: impl FnOnce(&mut MarkdownRuleOverrides)) -> String {
    let mut overrides = MarkdownRuleOverrides::default();
    set(&mut overrides);
    let cleaned = body_support::clean_accepted(content, &resolve(&overrides, None, None));
    assert_eq!(cleanup_content_with(content, &overrides), cleaned, "wrapper diverged from clean_body");
    cleaned
}

/// [`clean_with`] under the runtime defaults.
#[track_caller]
fn clean(content: &str) -> String {
    let cleaned = clean_with(content, |_| {});
    assert_eq!(cleanup_content(content), cleaned, "cleanup_content diverged from clean_body");
    cleaned
}

/// [`clean_with`] with `unwrap: true`, the old default soft-break stripping.
#[track_caller]
fn clean_unwrapped(content: &str) -> String {
    clean_with(content, unwrap)
}

/// [`clean_with`] with `indentation` set to `spaces` per level.
#[track_caller]
fn clean_indent(content: &str, spaces: usize) -> String {
    clean_with(content, |overrides| overrides.indentation = Leaf::Value(indentation(spaces)))
}

fn indentation(spaces: usize) -> Indentation {
    Indentation::from_spaces(spaces).expect("2, 4, or 8")
}

/// A named rule setting, for fixture tables.
type SetRules = fn(&mut MarkdownRuleOverrides);

fn unwrap(overrides: &mut MarkdownRuleOverrides) {
    overrides.unwrap = Leaf::Value(true);
}

/// The old `--compact` spacing.
fn compact(overrides: &mut MarkdownRuleOverrides) {
    overrides.list_item_padding = Leaf::Value(ListItemPadding::None);
}

/// The old `--loose` spacing.
fn loose(overrides: &mut MarkdownRuleOverrides) {
    overrides.list_item_padding = Leaf::Value(ListItemPadding::One);
}

/// The standalone helpers chained the way the removed
/// `cleanup_to_fixed_width` chained them.
fn strip_then_reflow(content: &str, width: usize) -> String {
    reflow_to_width(&strip_incidental_newlines(content), width)
}

// ==================== Blank Lines ====================
//
// Cleanup no longer normalizes blank lines: it neither inserts one after a
// heading, code block, or paragraph nor trims or collapses them.

#[test]
fn heading_directly_followed_by_paragraph_is_unchanged() {
    let content = "# Title\nParagraph text";
    assert_eq!(clean(content), content);
}

#[test]
fn consecutive_headings_are_unchanged() {
    let content = "# Header 1\n## Header 2";
    assert_eq!(clean(content), content);
}

#[test]
fn paragraph_directly_after_code_block_is_unchanged() {
    let content = "```rust\nfn main() {}\n```\nParagraph after";
    assert_eq!(clean(content), content);
}

#[test]
fn blank_line_after_list_is_kept() {
    let content = "* Item 1\n* Item 2\n\nParagraph after";
    assert_eq!(clean(content), content);
}

#[test]
fn blank_line_after_blockquote_is_kept() {
    let content = "> Quote\n\nParagraph after";
    assert_eq!(clean(content), content);
}

#[test]
fn existing_single_blank_lines_are_kept() {
    let content = "# Header\n\nSome text\n\n## Subheader";
    assert_eq!(clean(content), content);
}

#[test]
fn mixed_blank_line_spacing_is_unchanged() {
    let content = "# Title\nParagraph 1\n\nParagraph 2";
    assert_eq!(clean(content), content);
}

// ==================== Edge Cases ====================

#[test]
fn empty_content_is_unchanged() {
    assert_eq!(clean(""), "");
}

#[test]
fn plain_text_is_unchanged() {
    assert_eq!(clean("Just plain text"), "Just plain text");
}

#[test]
fn missing_final_newline_is_not_added() {
    let content = "# Title\n\nParagraph";
    assert_eq!(clean(content), content);
}

#[test]
fn multiple_trailing_newlines_are_kept() {
    let content = "# Title\n\nParagraph\n\n\n";
    assert_eq!(clean(content), content);
}

#[test]
fn multiple_paragraphs_are_unchanged() {
    let content = "Para 1\n\nPara 2\n\nPara 3";
    assert_eq!(clean(content), content);
}

#[test]
fn code_block_content_is_unchanged() {
    let content = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
    assert_eq!(clean(content), content);
}

#[test]
fn code_block_indentation_with_info_attributes_is_unchanged() {
    let content = "```ts title=\"Greet Function\"\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
    assert_eq!(clean(content), content);
}

#[test]
fn code_block_indentation_is_unchanged() {
    let content = "```ts\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
    assert_eq!(clean(content), content);
}

// ==================== Regressions ====================

#[test]
fn no_hard_break_is_introduced_between_blocks() {
    let content = "# Header\n\nParagraph\n\n## Another Header";
    assert_eq!(clean(content), content);
}

#[test]
fn code_fence_length_is_unchanged() {
    let content = "```rust\nfn main() {}\n```";
    assert_eq!(clean(content), content);
}

#[test]
fn unlabeled_fence_gets_default_language() {
    assert_eq!(clean("```\nsome code\n```"), "```text\nsome code\n```");
}

#[test]
fn fence_language_is_kept() {
    let content = "```rust\nfn main() {}\n```";
    assert_eq!(clean(content), content);
}

#[test]
fn every_unlabeled_fence_gets_default_language() {
    assert_eq!(
        clean("```\nfirst\n```\n\n```\nsecond\n```"),
        "```text\nfirst\n```\n\n```text\nsecond\n```"
    );
}

#[test]
fn indented_code_block_is_unchanged() {
    let content = "    indented code\n    more code";
    assert_eq!(clean(content), content);
}

// ==================== Smart Quotes (Regression) ====================

#[test]
fn straight_double_quotes_are_unchanged() {
    let content = r#"He said "hello" and "goodbye"."#;
    assert_eq!(clean(content), content);
}

#[test]
fn straight_single_quotes_are_unchanged() {
    let content = "It's a test and 'quoted text' here.";
    assert_eq!(clean(content), content);
}

#[test]
fn quotes_in_code_are_unchanged() {
    assert_eq!(clean("```\nlet s = \"hello\";\n```"), "```text\nlet s = \"hello\";\n```");
}
mod body_support;
mod fidelity_repro;
mod directive_fixed_points;
mod rules;
mod guard;
mod source_fidelity;
