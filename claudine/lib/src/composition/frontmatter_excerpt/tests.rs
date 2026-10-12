//! Tests for focused frontmatter excerpts.

use super::*;

// Note: written as one literal (no `\`-newline continuations) because those
// would strip the leading YAML indentation that the nesting tests rely on.
const DOC: &str = "---\ntitle: Example\niteration: 1\nsuccess:\n    message: \"done at {{review_file}}\"\n    effect: cheer\nfailure:\n    message: \"failed\"\n---\nbody\n";

#[test]
fn block_includes_delimiters_so_lines_match_file() {
    let block = capture_frontmatter_block(DOC).unwrap();
    assert!(block.starts_with("---\n"));
    assert!(block.ends_with("\n---"));
    // Line 1 is the opening delimiter, matching the source file.
    assert_eq!(block.lines().next(), Some("---"));
}

#[test]
fn block_none_without_closing_delimiter() {
    assert_eq!(capture_frontmatter_block("---\ntitle: x\nbody\n"), None);
}

#[test]
fn block_none_without_opening_delimiter() {
    assert_eq!(capture_frontmatter_block("title: x\n"), None);
}

#[test]
fn locate_top_level_key() {
    let block = capture_frontmatter_block(DOC).unwrap();
    // `iteration:` is the 3rd line of the file (after `---`, `title:`).
    assert_eq!(locate_property_line(&block, "iteration"), Some(3));
}

#[test]
fn locate_nested_key() {
    let block = capture_frontmatter_block(DOC).unwrap();
    // `success.message` is on line 5.
    assert_eq!(locate_property_line(&block, "success.message"), Some(5));
}

#[test]
fn locate_nested_key_under_second_parent() {
    let block = capture_frontmatter_block(DOC).unwrap();
    // `failure.message` is on line 8 — must not match `success.message`.
    assert_eq!(locate_property_line(&block, "failure.message"), Some(8));
}

#[test]
fn locate_nested_sequence_value() {
    let block = capture_frontmatter_block(
        "---\nsuccess:\n    stack:\n        - action:\n            - set:\n                metadata:\n                    files:\n                        - \"{{unknown_root}}\"\n---\n",
    )
    .unwrap();
    assert_eq!(
        locate_property_line(
            &block,
            "success.stack[0].action[0].set.metadata.files[0]"
        ),
        Some(8)
    );
}

#[test]
fn locate_absent_key_is_none() {
    let block = capture_frontmatter_block(DOC).unwrap();
    assert_eq!(locate_property_line(&block, "missing"), None);
    assert_eq!(locate_property_line(&block, "success.absent"), None);
}

#[test]
fn locate_does_not_match_value_substring() {
    let block = capture_frontmatter_block(DOC).unwrap();
    // `message` appears as a value-bearing key under success/failure but
    // never as a top-level key.
    assert_eq!(locate_property_line(&block, "message"), None);
}

/// A `:` inside a plain or quoted key is part of the key, not its separator.
#[test]
fn locate_a_key_holding_a_colon() {
    let block = capture_frontmatter_block(
        "---\na:b: 1\na: 2\nx:'y:\n    inner: v\n'p: q': 3\n---\nbody\n",
    )
    .unwrap();
    assert_eq!(locate_property_line(&block, "a:b"), Some(2));
    assert_eq!(locate_property_line(&block, "a"), Some(3));
    assert_eq!(locate_property_line(&block, "x:'y.inner"), Some(5));
    assert_eq!(locate_property_line(&block, "p: q"), Some(6));
}

#[test]
fn capture_returns_none_without_frontmatter() {
    assert!(FrontmatterExcerpt::capture("no frontmatter", "x", true).is_none());
}

#[test]
fn appendix_empty_when_not_tty() {
    let excerpt = FrontmatterExcerpt::capture(DOC, "success.message", false).unwrap();
    let term = Terminal::new_optimistic(80);
    assert_eq!(excerpt.render_appendix(&term), "");
}

#[test]
fn appendix_shows_yaml_when_tty() {
    let excerpt = FrontmatterExcerpt::capture(DOC, "success.message", true).unwrap();
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(excerpt.render_appendix(&term));
    assert!(rendered.contains("message"), "got: {rendered}");
    assert!(rendered.contains("iteration"), "got: {rendered}");
}

#[test]
fn appendix_plain_when_no_color() {
    let excerpt = FrontmatterExcerpt::capture(DOC, "success.message", true).unwrap();
    let term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::None)
        .build();
    let rendered = excerpt.render_appendix(&term);
    assert!(
        !rendered.contains('\x1b'),
        "plain appendix must have no escape bytes; got: {rendered:?}"
    );
}

// An inline `$schema` mapping whose `spec` type-string has a bad constraint
// separator (`,` instead of `;`). `spec` is the 3rd file line.
const SCHEMA_DOC: &str =
    "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nbody\n";

#[test]
fn schema_span_highlights_offending_property_line() {
    // The span points into the single-line `spec` type string, so it must
    // land on the property's own line (line 3), not the `$schema` parent.
    let excerpt =
        FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, Some("$schema.spec"), 13, true)
            .unwrap();
    assert_eq!(excerpt.highlight_line(), Some(3));
}

#[test]
fn schema_span_does_not_highlight_unrelated_line() {
    // The top-level `spec: "x"` value on line 4 must never be highlighted in
    // place of the `$schema.spec` type-string line.
    let excerpt =
        FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, Some("$schema.spec"), 13, true)
            .unwrap();
    assert_ne!(excerpt.highlight_line(), Some(4));
}

#[test]
fn schema_span_falls_back_to_schema_parent_without_property() {
    // A structural failure with no real property name falls back to the
    // `$schema:` parent line (line 2).
    let excerpt =
        FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, None, 0, true).unwrap();
    assert_eq!(excerpt.highlight_line(), Some(2));
}

#[test]
fn value_line_offset_zero_for_single_line_value() {
    let block = capture_frontmatter_block(SCHEMA_DOC).unwrap();
    // Any in-range span into the single-line `spec` value crosses no newline.
    assert_eq!(value_line_offset(&block, 3, 0), 0);
    assert_eq!(value_line_offset(&block, 3, 13), 0);
}

#[test]
fn value_line_offset_counts_newlines_across_continuation_lines() {
    // Defensive mechanic: when a value's reconstructed text spans physical
    // lines (a YAML block scalar), the offset counts the line-breaks the span
    // crosses. Real SimplifiedSchema type strings are single-line, so this
    // path returns 0 in practice; the test pins the multi-line arithmetic.
    let doc = "---\n$schema:\n    spec: a\n      b\n      c\n---\nbody\n";
    let block = capture_frontmatter_block(doc).unwrap();
    // Reconstructed value text for `spec` is "a\nb\nc"; a span past the first
    // newline lands one continuation line down, past the second lands two.
    assert_eq!(value_line_offset(&block, 3, 0), 0);
    assert_eq!(value_line_offset(&block, 3, "a\nb".len()), 1);
    assert_eq!(value_line_offset(&block, 3, "a\nb\nc".len()), 2);
}

#[test]
fn value_line_offset_reads_past_a_content_colon_in_the_key() {
    let doc = "---\n$schema:\n    a:b: x\n      y\n---\nbody\n";
    let block = capture_frontmatter_block(doc).unwrap();
    // The value is "x\ny", not "b: x\ny".
    assert_eq!(value_line_offset(&block, 3, "x\ny".len()), 1);
}

#[test]
fn schema_span_appendix_withheld_when_not_tty() {
    let excerpt =
        FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, Some("$schema.spec"), 13, false)
            .unwrap();
    let term = Terminal::new_optimistic(80);
    assert_eq!(excerpt.render_appendix(&term), "");
}

const NEAR_MISS_DOC: &str = "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n";

#[test]
fn capture_line_recognizes_four_dash_fence() {
    let excerpt = FrontmatterExcerpt::capture_line(NEAR_MISS_DOC, 1, true).unwrap();
    assert_eq!(excerpt.highlight_line(), Some(1));
    // Both fences fall inside line 1's window, so the region spans the block.
    assert_eq!(excerpt.line_spans(), vec![(1, 4)]);
    assert!(excerpt.regions[0].text.starts_with("----\n"), "must include opening fence");
    assert!(excerpt.regions[0].text.ends_with("\n----"), "must include closing fence");
}

#[test]
fn capture_line_none_for_plain_prose() {
    assert!(FrontmatterExcerpt::capture_line("no frontmatter here\n", 1, true).is_none());
}

#[test]
fn capture_line_windows_a_valid_three_dash_block() {
    // A located YAML parse error on line 5 of a `---` block.
    let excerpt = FrontmatterExcerpt::capture_line(DOC, 5, true).unwrap();
    assert_eq!(excerpt.highlighted_lines(), vec![5]);
    assert_eq!(excerpt.line_spans(), vec![(2, 8)]);
}

#[test]
fn capture_line_none_outside_the_block() {
    assert!(FrontmatterExcerpt::capture_line(DOC, 10, true).is_none());
    assert!(FrontmatterExcerpt::capture_line(DOC, 0, true).is_none());
}

#[test]
fn capture_line_appendix_empty_when_not_tty() {
    let excerpt = FrontmatterExcerpt::capture_line(NEAR_MISS_DOC, 1, false).unwrap();
    let term = Terminal::new_optimistic(80);
    assert_eq!(excerpt.render_appendix(&term), "");
}

#[test]
fn capture_line_appendix_highlights_fence_line() {
    let excerpt = FrontmatterExcerpt::capture_line(NEAR_MISS_DOC, 1, true).unwrap();
    assert_eq!(excerpt.highlight_line(), Some(1));
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(excerpt.render_appendix(&term));
    assert!(rendered.contains("name:"), "yaml block missing: {rendered}");
    assert!(rendered.contains("----"), "fence line missing: {rendered}");
}

// A mid-file key (`settings.target`, line 10) far from both block ends.
const MID_DOC: &str = "---\nagent: codex\nsettings:\n  a: 1\n  b: 2\n  c: 3\n  d: 4\n  e: 5\n  f: 6\n  target: x\n  g: 7\n  h: 8\n  i: 9\n  j: 10\n  k: 11\ntail: 1\n---\nbody\n";

// A frozen copy of `prompts/clarify.md`, intentionally not kept in step with the
// live prompt.
const CLARIFY: &str = include_str!("../../../tests/fixtures/frozen_prompts/prompts/clarify.md");

fn plain_term() -> Terminal {
    Terminal::builder()
        .width(100)
        .color_depth(ColorDepth::None)
        .build()
}

/// The 1-based line of the first line of `text` that starts with `prefix`.
fn line_starting_with(text: &str, prefix: &str) -> usize {
    text.lines()
        .position(|line| line.starts_with(prefix))
        .map(|idx| idx + 1)
        .unwrap_or_else(|| panic!("no line starts with {prefix:?}"))
}

#[test]
fn mid_file_property_shows_three_lines_either_side_plus_its_ancestor() {
    let excerpt = FrontmatterExcerpt::capture(MID_DOC, "settings.target", true).unwrap();

    assert_eq!(excerpt.highlighted_lines(), vec![10]);
    // The ±3 window is 7 lines; `settings:` (line 3) is its ancestor.
    assert_eq!(excerpt.line_spans(), vec![(3, 3), (7, 13)]);

    let rendered = excerpt.render_appendix(&plain_term());
    assert!(!rendered.contains("agent"), "line 2 is outside the focus: {rendered}");
    assert!(!rendered.contains("tail"), "line 16 is outside the focus: {rendered}");
    assert!(rendered.contains("settings:"), "{rendered}");
}

#[test]
fn caller_input_problem_highlights_its_arm_declaration_in_clarify() {
    // `/spec` names a caller input, not a top-level key; its declaration is
    // the first `$schema` union arm.
    let spec_line = line_starting_with(CLARIFY, "    - spec:");
    let schema_line = line_starting_with(CLARIFY, "$schema:");

    let excerpt = FrontmatterExcerpt::capture_schema_properties(CLARIFY, &["spec"], true)
        .expect("the `spec` declaration is locatable");

    assert_eq!(excerpt.highlighted_lines(), vec![spec_line]);
    let (first, last) = (excerpt.line_spans()[0].0, excerpt.line_spans().last().unwrap().1);
    assert!(first <= schema_line, "the `$schema:` parent is shown");
    assert!(last <= spec_line + EXCERPT_CONTEXT_LINES, "no line past the window");

    let block_lines = capture_frontmatter_block(CLARIFY).unwrap().lines().count();
    let shown: usize = excerpt.line_spans().iter().map(|(s, e)| e - s + 1).sum();
    assert!(shown < block_lines, "never the whole {block_lines}-line block: {shown}");
}

#[test]
fn schema_property_shows_every_arm_and_a_same_named_top_level_key() {
    let doc = "---\n$schema:\n  - spec: file(match(**/*spec*.md); eager)\n    doc: file\n    a: 1\n    b: 2\n    c: 3\n    d: 4\n  - spec: file(match(fixes/**/spec.md); eager)\n    doc: file\n    e: 1\n    f: 2\n    g: 3\n    h: 4\n    i: 5\n    j: 6\nspec: fix\n---\nbody\n";

    let excerpt = FrontmatterExcerpt::capture_schema_properties(doc, &["spec"], true).unwrap();

    assert_eq!(excerpt.highlighted_lines(), vec![3, 9, 17]);
    assert_eq!(excerpt.line_spans(), vec![(1, 12), (14, 18)]);
}

#[test]
fn schema_property_under_an_inline_mapping_schema_is_located() {
    let excerpt =
        FrontmatterExcerpt::capture_schema_properties(SCHEMA_DOC, &["spec"], true).unwrap();
    // The `$schema.spec` declaration (line 3) and the top-level value (line 4).
    assert_eq!(excerpt.highlighted_lines(), vec![3, 4]);
}

#[test]
fn unlocatable_property_gives_no_excerpt() {
    assert_eq!(FrontmatterExcerpt::capture(DOC, "nope", true), None);
    assert_eq!(FrontmatterExcerpt::capture(DOC, "success.nope", true), None);
    assert_eq!(
        FrontmatterExcerpt::capture_schema_properties(CLARIFY, &["nope"], true),
        None
    );
    assert_eq!(
        FrontmatterExcerpt::capture_properties(DOC, &[] as &[&str], true),
        None
    );
    // An absent key is not rescued by the `$schema` fallback either.
    assert_eq!(
        FrontmatterExcerpt::capture_schema_span(DOC, Some("$schema.spec"), 0, true),
        None
    );
}

#[test]
fn two_problems_give_two_regions_with_an_elision_line() {
    let excerpt =
        FrontmatterExcerpt::capture_properties(MID_DOC, &["agent", "settings.k"], true).unwrap();

    assert_eq!(excerpt.line_spans(), vec![(1, 5), (12, 17)]);
    let rendered = excerpt.render_appendix(&plain_term());
    let before = rendered.find("agent").unwrap();
    let elision = rendered.find('⋮').expect("an elision line between regions");
    let after = rendered.find("k: 11").unwrap();
    assert!(before < elision && elision < after, "{rendered}");
    assert_eq!(rendered.matches('⋮').count(), 1, "{rendered}");
}

#[test]
fn rendered_gutter_numbers_match_source_lines() {
    let excerpt =
        FrontmatterExcerpt::capture_properties(MID_DOC, &["agent", "settings.k"], true).unwrap();
    // Line numbers are drawn only when the terminal has color.
    let rendered = strip_escape_codes(excerpt.render_appendix(&Terminal::new_optimistic(100)));

    for (number, text) in MID_DOC.lines().enumerate().map(|(idx, text)| (idx + 1, text)) {
        let shown = (1..=5).contains(&number) || (12..=17).contains(&number);
        let row = rendered.lines().find(|row| {
            row.trim_start().starts_with(&format!("{number} │ {text}"))
        });
        assert_eq!(row.is_some(), shown, "line {number} ({text:?}) in: {rendered}");
    }

    // The elision sits in the gutter column of the region below it.
    let rows: Vec<&str> = rendered.lines().collect();
    let elision = rows.iter().position(|row| row.contains('⋮')).unwrap();
    let column = |row: &str, glyph: char| row.chars().position(|c| c == glyph);
    let next_gutter = rows[elision..].iter().find_map(|row| column(row, '│'));
    assert_eq!(column(rows[elision], '⋮'), next_gutter, "{rendered}");
}

#[test]
fn a_block_the_window_already_covers_renders_whole() {
    // Every line of a five-line block is within ±3 of line 3.
    let doc = "---\na: 1\nb: 2\nc: 3\n---\nbody\n";
    let excerpt = FrontmatterExcerpt::capture(doc, "b", true).unwrap();
    assert_eq!(excerpt.line_spans(), vec![(1, 5)]);
}

#[test]
fn item_keys_after_the_first_are_siblings_not_children() {
    let block = capture_frontmatter_block(CLARIFY).unwrap();
    let spec = line_starting_with(CLARIFY, "    - spec:");
    let design = line_starting_with(CLARIFY, "    - design:");

    assert_eq!(locate_property_line(&block, "$schema[0]"), Some(spec));
    assert_eq!(locate_property_line(&block, "$schema[0].spec"), Some(spec));
    assert_eq!(locate_property_line(&block, "$schema[0].doc"), Some(spec + 1));
    assert_eq!(locate_property_line(&block, "$schema[1].doc"), Some(design + 1));
    assert_eq!(locate_property_line(&block, "$schema[0].spec.doc"), None);
}

