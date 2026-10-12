use super::*;

// ==================== Blockquotes ====================
//
// Blockquote prefixes are authored bytes; cleanup neither respaces nor strips
// them.

#[test]
fn blockquotes_are_unchanged() {
    for content in [
        "> Simple quote",
        "> > Nested quote",
        "> > > Triple nested",
        "# Header\n\n> Quote after header",
        "> Ut faucibus mauris mauris, sed tincidunt augue hendrerit eu. In ultrices ultrices commodo.",
        "> Code:   let x = 1",
        "> Line 1\n> Line 2\n> Line 3",
    ] {
        assert_eq!(clean(content), content);
    }
}

#[test]
fn unwrap_joins_blockquote_lines_and_keeps_prefix() {
    assert_eq!(clean_unwrapped("> Line 1\n> Line 2\n> Line 3"), "> Line 1 Line 2 Line 3");
}

#[test]
fn blank_blockquote_line_before_attribution_is_kept() {
    let content = "> Some quoted content\n>\n> — Wikipedia";
    assert_eq!(clean(content), content);
    assert_eq!(clean_unwrapped(content), content);
}
