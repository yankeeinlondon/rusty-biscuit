//! Tests for the context-aware lexical source map: line splitting and
//! classification, mapping/sequence entries, flow regions, block scalars,
//! comments, anchors, document markers, quoted scalars, and context paths.

use super::super::scan::{AnchorKind, FlowKind, LineKind, MarkerKind, PathSegment, QuoteStyle, SourceMap};

#[test]
fn test_line_splitting_lf() {
    let map = SourceMap::new("a\nbb\nccc");
    assert_eq!(map.lines().len(), 3);
    assert_eq!(map.lines()[0].span, 0..2);
    assert_eq!(map.lines()[0].content, 0..1);
    assert_eq!(map.lines()[2].span, 5..8);
    assert_eq!(map.lines()[2].content, 5..8);
}

#[test]
fn test_line_splitting_crlf_and_lone_cr() {
    let map = SourceMap::new("a\r\nb\rc\n");
    assert_eq!(map.lines().len(), 3);
    assert_eq!(map.lines()[0].span, 0..3);
    assert_eq!(map.lines()[0].content, 0..1);
    assert_eq!(map.lines()[1].span, 3..5);
    assert_eq!(map.lines()[1].content, 3..4);
    assert_eq!(map.lines()[2].span, 5..7);
    assert_eq!(map.lines()[2].content, 5..6);
}

#[test]
fn test_line_classification() {
    let map = SourceMap::new("# comment\n\n---\nkey: value\n...\n");
    let kinds: Vec<_> = map.lines().iter().map(|line| line.kind).collect();
    assert_eq!(
        kinds,
        vec![
            LineKind::Comment,
            LineKind::Blank,
            LineKind::DocumentStart,
            LineKind::Content,
            LineKind::DocumentEnd,
        ]
    );
    assert_eq!(map.markers().len(), 2);
    assert_eq!(map.markers()[0].kind, MarkerKind::Start);
    assert_eq!(map.markers()[1].kind, MarkerKind::End);
}

#[test]
fn test_mapping_entry_extraction() {
    let map = SourceMap::new("key :  value\n");
    let entry = map.entry(0);
    assert!(entry.dash.is_none());
    let mapping = entry.mapping.as_ref().expect("mapping entry");
    assert_eq!(mapping.key, 0..3);
    assert_eq!(mapping.colon, 4);
    assert_eq!(mapping.value, Some(7..12));
}

#[test]
fn test_mapping_colon_requires_following_whitespace() {
    // `host:localhost` is a plain scalar, not a mapping.
    let map = SourceMap::new("host:localhost\n");
    assert!(map.entry(0).mapping.is_none());
    // The colon inside a URL is not a mapping colon either.
    let map = SourceMap::new("url: http://example.com\n");
    let mapping = map.entry(0).mapping.as_ref().expect("mapping entry");
    assert_eq!(mapping.key, 0..3);
    assert_eq!(mapping.value, Some(5..23));
}

#[test]
fn test_sequence_entry_extraction() {
    let map = SourceMap::new("- item\n-80\n- \n");
    let first = map.entry(0);
    assert_eq!(first.dash, Some(0));
    assert_eq!(first.dash_value, Some(2..6));
    // `-80` is a plain scalar, not a sequence entry.
    assert!(map.entry(1).dash.is_none());
    // A bare dash is a sequence entry with no value.
    let third = map.entry(2);
    assert_eq!(third.dash, Some(11));
    assert_eq!(third.dash_value, None);
}

#[test]
fn test_sequence_entry_with_inline_mapping() {
    let map = SourceMap::new("- name: value\n");
    let entry = map.entry(0);
    assert_eq!(entry.dash, Some(0));
    assert_eq!(entry.dash_value, Some(2..13));
    let mapping = entry.mapping.as_ref().expect("inline mapping");
    assert_eq!(mapping.key, 2..6);
    assert_eq!(mapping.value, Some(8..13));
}

#[test]
fn test_comment_trimmed_from_value_region() {
    let map = SourceMap::new("key: value # note\n");
    let mapping = map.entry(0).mapping.as_ref().expect("mapping entry");
    assert_eq!(mapping.value, Some(5..10));
    assert_eq!(map.comments().len(), 1);
    assert_eq!(map.comments()[0], 11..17);
}

#[test]
fn test_flow_regions_nested_and_unclosed() {
    let map = SourceMap::new("a: [1, {b: 2}]\n");
    assert_eq!(map.flow_regions().len(), 2);
    let outer = &map.flow_regions()[0];
    assert_eq!(outer.kind, FlowKind::Sequence);
    assert!(outer.closed);
    assert_eq!(outer.span, 3..14);
    let inner = &map.flow_regions()[1];
    assert_eq!(inner.kind, FlowKind::Mapping);
    assert_eq!(inner.span, 7..13);

    let unclosed = SourceMap::new("a: [1,\n");
    assert_eq!(unclosed.flow_regions().len(), 1);
    assert!(!unclosed.flow_regions()[0].closed);
}

#[test]
fn test_brackets_inside_quotes_are_not_flow() {
    let map = SourceMap::new("a: \"[not flow]\"\n");
    assert!(map.flow_regions().is_empty());
}

#[test]
fn test_block_scalar_detection_and_content_reclassification() {
    let source = "script: |\n  echo hi\n  # not a comment\nnext: 1\n";
    let map = SourceMap::new(source);
    assert_eq!(map.block_scalars().len(), 1);
    assert_eq!(map.block_scalars()[0].header_line, 0);
    assert_eq!(map.lines()[1].kind, LineKind::BlockContent);
    // The `#` line inside the block is content, not a comment.
    assert_eq!(map.lines()[2].kind, LineKind::BlockContent);
    assert!(map.comments().is_empty());
    assert_eq!(map.lines()[3].kind, LineKind::Content);
}

#[test]
fn test_block_scalar_folded_with_modifiers() {
    let map = SourceMap::new("text: >- # folded\n  some text\n");
    assert_eq!(map.block_scalars().len(), 1);
    assert_eq!(map.lines()[1].kind, LineKind::BlockContent);
}

#[test]
fn test_root_level_block_scalar() {
    let map = SourceMap::new("|\n  root text\n");
    assert_eq!(map.block_scalars().len(), 1);
    assert_eq!(map.lines()[1].kind, LineKind::BlockContent);
}

#[test]
fn test_quoted_scalar_styles() {
    let map = SourceMap::new("a: 'it''s'\nb: \"x\\n\"\n");
    let quoted = map.quoted_scalars();
    assert_eq!(quoted.len(), 2);
    assert_eq!(quoted[0].style, QuoteStyle::Single);
    assert_eq!(quoted[0].span, 3..10);
    assert_eq!(quoted[1].style, QuoteStyle::Double);
    assert!(quoted.iter().all(|scalar| scalar.closed));
}

#[test]
fn test_apostrophe_inside_plain_scalar_is_not_a_quote() {
    let map = SourceMap::new("owner: Ken's account\n");
    assert!(map.quoted_scalars().is_empty());
}

#[test]
fn test_anchor_and_alias_recording() {
    let map = SourceMap::new("anchor: &x 1\nalias: *x\ncalc: 2 * 3\n");
    let anchors = map.anchors();
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].kind, AnchorKind::Anchor);
    assert_eq!(&"anchor: &x 1\nalias: *x\ncalc: 2 * 3\n"[anchors[0].name.clone()], "x");
    assert_eq!(anchors[1].kind, AnchorKind::Alias);
    // `2 * 3` is scalar content: no alias recorded for it.
}

#[test]
fn test_block_value_context_mapping() {
    let source = "a:\n  b:\n    title: @x\n";
    let map = SourceMap::new(source);
    let context = map.block_value_context(source, 2).expect("context");
    assert_eq!(
        context.path,
        vec![
            PathSegment::Key("a".to_string()),
            PathSegment::Key("b".to_string()),
            PathSegment::Key("title".to_string()),
        ]
    );
    assert_eq!(&source[context.lexeme.clone()], "@x");
}

#[test]
fn test_block_value_context_sequence_index() {
    let source = "items:\n  - one\n  - @two\n";
    let map = SourceMap::new(source);
    let context = map.block_value_context(source, 2).expect("context");
    assert_eq!(
        context.path,
        vec![
            PathSegment::Key("items".to_string()),
            PathSegment::Index(1),
        ]
    );
}

#[test]
fn test_block_value_context_inline_mapping_in_sequence() {
    let source = "items:\n  - name: a\n    title: @x\n";
    let map = SourceMap::new(source);
    let context = map.block_value_context(source, 2).expect("context");
    assert_eq!(
        context.path,
        vec![
            PathSegment::Key("items".to_string()),
            PathSegment::Index(0),
            PathSegment::Key("title".to_string()),
        ]
    );
}

#[test]
fn test_block_value_context_lexeme_includes_comment() {
    // The bounded grammar takes the lexeme to end of line; the ` #` check
    // downstream rejects the ambiguity.
    let source = "title: @x # note\n";
    let map = SourceMap::new(source);
    let context = map.block_value_context(source, 0).expect("context");
    assert_eq!(&source[context.lexeme.clone()], "@x # note");
}

#[test]
fn test_line_at_byte() {
    let map = SourceMap::new("ab\ncd\n");
    assert_eq!(map.line_at_byte(0), Some(0));
    assert_eq!(map.line_at_byte(2), Some(0));
    assert_eq!(map.line_at_byte(3), Some(1));
    assert_eq!(map.line_at_byte(6), Some(1));
}

#[test]
fn test_multibyte_spans() {
    let map = SourceMap::new("key: 日本語\n");
    let mapping = map.entry(0).mapping.as_ref().expect("mapping entry");
    assert_eq!(mapping.value, Some(5..14));
}
