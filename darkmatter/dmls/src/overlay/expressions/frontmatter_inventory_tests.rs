//! The frontmatter interpolation inventory: scalar-style projection, the
//! nested-span lint in authored coordinates, and style-encoded rewrites.

use super::*;

const INCIDENT: &str =
    include_str!("../../../../../claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md");
const COMMIT: &str =
    include_str!("../../../../../claudine/cli/tests/fixtures/nested_span_regression/commit.md");

fn ast(text: &str) -> FrontmatterAst {
    FrontmatterAst::parse(text).expect("frontmatter").ast.expect("parses")
}

/// `(dotted path, authored outer span text, exact)` for every interpolation.
fn inventory(text: &str) -> Vec<(String, String, bool)> {
    let ast = ast(text);
    frontmatter_interpolations(text, &ast)
        .iter()
        .map(|span| {
            (
                span.entry.dotted.clone(),
                text[span.outer_span()].to_string(),
                span.projection().is_exact(),
            )
        })
        .collect()
}

/// Applies the single rewrite each nested-span interpolation offers and
/// returns the edited document.
fn apply_rewrites(text: &str, mode: ParseMode) -> String {
    let ast = ast(text);
    let mut edits: Vec<(SourceSpan, String)> = frontmatter_interpolations(text, &ast)
        .iter()
        .filter_map(|span| {
            // Every lint of one expression carries the same rewrite: apply once.
            span.nested_span_lints(text, mode).into_iter().find_map(|lint| lint.replacement)
        })
        .collect();
    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut edited = text.to_string();
    for (span, replacement) in edits {
        edited.replace_range(span, &replacement);
    }
    edited
}

fn nested_lint_count(text: &str) -> usize {
    let ast = ast(text);
    frontmatter_interpolations(text, &ast)
        .iter()
        .map(|span| span.nested_span_lints(text, ParseMode::Interpolation).len())
        .sum()
}

#[test]
fn plain_and_quoted_spans_project_exactly() {
    let text = concat!(
        "---\n",
        "plain: done in {{ ctx.repo }} now\n",
        "single: 'it''s {{ ctx.repo }}'\n",
        "double: \"a\\n\\\"q\\\" {{  ctx.repo }}\"\n",
        "whole: \"{{ ctx.repo }}\"\n",
        "---\n\nbody {{ not.frontmatter }}\n",
    );
    let ast = ast(text);
    let spans = frontmatter_interpolations(text, &ast);
    let summary: Vec<(&str, &str, &str, bool)> = spans
        .iter()
        .map(|span| {
            (
                span.entry.dotted.as_str(),
                &text[span.outer_span()],
                &text[span.inner_span().expect("exact")],
                span.whole_value,
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            ("plain", "{{ ctx.repo }}", "ctx.repo", false),
            ("single", "{{ ctx.repo }}", "ctx.repo", false),
            ("double", "{{  ctx.repo }}", "ctx.repo", false),
            ("whole", "{{ ctx.repo }}", "ctx.repo", true),
        ]
    );
    assert!(spans.iter().all(|span| span.text == "ctx.repo"));
    // Sub-expression projection lands on authored bytes.
    let root = spans[2].project(0..3).unwrap();
    assert_eq!(&text[root], "ctx");
}

#[test]
fn incident_literal_blocks_range_each_nested_span_in_authored_coordinates() {
    let ast = ast(INCIDENT);
    let spans = frontmatter_interpolations(INCIDENT, &ast);
    let says: Vec<&FrontmatterInterpolation<'_>> =
        spans.iter().filter(|span| span.entry.key == "say").collect();
    assert_eq!(says.len(), 2);
    for say in &says {
        assert!(say.whole_value, "{} is a whole-value span", say.entry.dotted);
        assert!(say.projection().is_exact());
        assert!(INCIDENT[say.outer_span()].starts_with("{{\n"));
    }
    let success = says[0].nested_span_lints(INCIDENT, ParseMode::Interpolation);
    let ranged: Vec<&str> = success.iter().map(|lint| &INCIDENT[lint.range.clone()]).collect();
    assert_eq!(ranged, vec!["{{ctx.area}}", "{{ctx.repo_name}}"]);
    assert_eq!(success[1].nested, "ctx.repo_name");
    let failure = says[1].nested_span_lints(INCIDENT, ParseMode::Interpolation);
    let ranged: Vec<&str> = failure.iter().map(|lint| &INCIDENT[lint.range.clone()]).collect();
    assert_eq!(
        ranged,
        vec![
            "{{ title_case(without_date(parent_dir(spec))) }}",
            "{{ctx.area}}",
            "{{ title_case(without_date(parent_dir(spec))) }}",
            "{{ctx.repo_name}}",
        ]
    );
    // Every lint of one expression offers the same authored replacement.
    let replacements: Vec<_> = success.iter().map(|lint| lint.replacement.clone()).collect();
    assert!(replacements[0].is_some());
    assert!(replacements.iter().all(|replacement| *replacement == replacements[0]));
}

#[test]
fn incident_rewrite_preserves_block_indicator_indentation_and_untouched_bytes() {
    let edited = apply_rewrites(INCIDENT, ParseMode::Interpolation);
    assert_ne!(edited, INCIDENT);
    assert_eq!(nested_lint_count(INCIDENT), 6);
    assert_eq!(nested_lint_count(&edited), 0, "{edited}");
    let ast = ast(&edited);
    let say = ast.entry_by_dotted("success.say").unwrap();
    let authored = &edited[say.value_span.clone()];
    assert!(authored.starts_with("|-\n        {{\n        ctx.area\n"), "{authored}");
    assert!(
        authored.contains(r#"? "The review of the draft specification file in " + ctx.area + " has completed""#),
        "{authored}"
    );
    // Everything outside the two say blocks is byte-identical.
    let prefix_end = INCIDENT.find("success:").unwrap();
    assert_eq!(&edited[..prefix_end], &INCIDENT[..prefix_end]);
    assert!(edited.ends_with(&INCIDENT[INCIDENT.find("    message: \"💥").unwrap()..]));
    // Idempotent: a second pass offers nothing further.
    assert_eq!(apply_rewrites(&edited, ParseMode::Interpolation), edited);
}

#[test]
fn commit_fixture_literal_with_escaped_newline_is_single_line_and_rewritable() {
    let ast = ast(COMMIT);
    let spans = frontmatter_interpolations(COMMIT, &ast);
    let resides = spans.iter().find(|span| span.entry.key == "resides_in").unwrap();
    let lints = resides.nested_span_lints(COMMIT, ParseMode::Interpolation);
    let ranged: Vec<&str> = lints.iter().map(|lint| &COMMIT[lint.range.clone()]).collect();
    assert_eq!(
        ranged,
        vec![
            "{{ctx.dirty_package_areas}}",
            "{{length(ctx.dirty_package_areas)}}",
            "{{as_unordered_list(ctx.dirty_package_areas)}}",
        ]
    );
    // The authored `\n` is two characters, not a line break.
    assert!(lints.iter().all(|lint| lint.replacement.is_some()));
    let edited = apply_rewrites(COMMIT, ParseMode::Interpolation);
    assert_eq!(nested_lint_count(&edited), 0);
    assert!(edited.contains(r"' package area'"), "{edited}");
    assert!(edited.contains(r":\n '"), "{edited}");
    // List items are reached too: the stack message lives inside a sequence.
    assert!(inventory(COMMIT).iter().any(|(dotted, _, _)| dotted == "initialize.stack[0].action[0].message"));
}

#[test]
fn quoted_rewrites_are_encoded_for_their_scalar_style() {
    let single = "---\nsay: '{{ ok ? \"a {{ x }}\" : ''b'' }}'\n---\n\nbody\n";
    let edited = apply_rewrites(single, ParseMode::Interpolation);
    assert_eq!(edited, "---\nsay: '{{ ok ? \"a \" + x : ''b'' }}'\n---\n\nbody\n");
    assert_eq!(ast(&edited).entry_by_dotted("say").unwrap().scalar.as_deref(), Some("{{ ok ? \"a \" + x : 'b' }}"));
    assert_eq!(nested_lint_count(&edited), 0);

    let double = "---\nsay: \"{{ ok ? 'a {{ x }}' : \\\"b\\\" }}\"\n---\n\nbody\n";
    let edited = apply_rewrites(double, ParseMode::Interpolation);
    assert_eq!(edited, "---\nsay: \"{{ ok ? 'a ' + x : \\\"b\\\" }}\"\n---\n\nbody\n");
    assert_eq!(ast(&edited).entry_by_dotted("say").unwrap().scalar.as_deref(), Some("{{ ok ? 'a ' + x : \"b\" }}"));
    assert_eq!(nested_lint_count(&edited), 0);

    let plain = "---\nsay: note {{ 'a {{ y }}' }}\n---\n\nbody\n";
    let edited = apply_rewrites(plain, ParseMode::Interpolation);
    assert_eq!(edited, "---\nsay: note {{ 'a ' + y }}\n---\n\nbody\n");
    assert_eq!(nested_lint_count(&edited), 0);
}

#[test]
fn folded_tagged_and_multiline_scalars_range_whole_and_offer_no_rewrite() {
    let text = concat!(
        "---\n",
        "folded: >-\n  {{ ok ? \"a {{ x }}\" : \"b\" }}\n",
        "tagged: !!str \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "multiline: \"{{ ok ? 'a {{ x }}'\n  : 'b' }}\"\n",
        "---\n\nbody\n",
    );
    let ast = ast(text);
    let spans = frontmatter_interpolations(text, &ast);
    assert_eq!(spans.len(), 3);
    for span in &spans {
        assert!(!span.projection().is_exact(), "{}", span.entry.dotted);
        assert_eq!(span.outer_span(), span.entry.value_span, "{}", span.entry.dotted);
        assert_eq!(span.inner_span(), None);
        let lints = span.nested_span_lints(text, ParseMode::Interpolation);
        assert_eq!(lints.len(), 1, "{}", span.entry.dotted);
        assert_eq!(lints[0].range, span.entry.value_span);
        assert_eq!(lints[0].replacement, None);
    }
}

#[test]
fn literal_spanning_a_line_break_keeps_its_range_but_offers_no_rewrite() {
    let text = "---\nsay: |-\n  {{ ok ? \"done {{ x }}\n    later\" : \"b\" }}\n---\n\nbody\n";
    let ast = ast(text);
    let spans = frontmatter_interpolations(text, &ast);
    let lints = spans[0].nested_span_lints(text, ParseMode::Interpolation);
    assert_eq!(lints.len(), 1);
    assert_eq!(&text[lints[0].range.clone()], "{{ x }}");
    assert_eq!(lints[0].replacement, None);
}

#[test]
fn aliases_items_and_block_comments_are_handled_passively() {
    let text = concat!(
        "---\n",
        "anchor: &shared \"{{ x }}\"\n",
        "alias: *shared\n",
        "items:\n  - \"{{ y }}\"\n  - message: hi {{ z }}\n",
        "commented: |- # {{ not.an.expression }}\n  {{ w }}\n",
        "no_span: plain text\n",
        "---\n\nbody\n",
    );
    assert_eq!(
        inventory(text),
        vec![
            ("anchor".to_string(), "{{ x }}".to_string(), true),
            ("items[0]".to_string(), "{{ y }}".to_string(), true),
            ("items[1].message".to_string(), "{{ z }}".to_string(), true),
            ("commented".to_string(), "{{ w }}".to_string(), true),
        ]
    );
}

#[test]
fn backslash_escaped_spans_are_not_inventoried() {
    let text = "---\nsay: 'Use \\{{ name }} literally, but {{ real }}'\n---\n\nbody\n";
    let found = inventory(text);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, "{{ real }}");
}
