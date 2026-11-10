//! `source_position`: byte offset → 1-based line and character column.

use biscuit_file::source_position;

#[test]
fn first_byte_and_end_of_text_are_valid_positions() {
    assert_eq!(source_position("", 0), Some((1, 1)));
    assert_eq!(source_position("ab\ncd", 0), Some((1, 1)));
    assert_eq!(source_position("ab\ncd", 5), Some((2, 3)));
    assert_eq!(source_position("ab\n", 3), Some((2, 1)));
}

#[test]
fn multibyte_characters_before_the_target_count_as_one_column_each() {
    let text = "a: 1\n名前: é, n: bad";
    let offset = text.find("bad").unwrap();
    // `名前: é, n: ` is ten characters but fifteen bytes.
    assert_eq!(source_position(text, offset), Some((2, 11)));
}

#[test]
fn crlf_twin_gives_the_same_line_and_column_as_lf() {
    let lf = "x: 1\ny: [1, two]\nz: é\n";
    let crlf = lf.replace('\n', "\r\n");
    for token in ["two", "é", "1, two", "x"] {
        let in_lf = source_position(lf, lf.find(token).unwrap());
        let in_crlf = source_position(&crlf, crlf.find(token).unwrap());
        assert_eq!(in_lf, in_crlf, "{token}");
    }
}

#[test]
fn the_lf_of_a_crlf_sits_where_the_lf_twin_line_ends() {
    assert_eq!(source_position("ab\r\ncd", 3), Some((1, 3)));
    assert_eq!(source_position("ab\ncd", 2), Some((1, 3)));
    assert_eq!(source_position("ab\r\ncd", 2), Some((1, 3)));
    assert_eq!(source_position("ab\r\ncd", 4), Some((2, 1)));
}

#[test]
fn a_lone_cr_is_an_ordinary_character_and_breaks_no_line() {
    let text = "ab\rcd\ne";
    assert_eq!(source_position(text, text.find('c').unwrap()), Some((1, 4)));
    assert_eq!(source_position(text, text.find('e').unwrap()), Some((2, 1)));
    // A CR at the end of the text, with no LF after it, still counts.
    assert_eq!(source_position("ab\r", 3), Some((1, 4)));
}

#[test]
fn offsets_past_the_end_or_inside_a_character_have_no_position() {
    assert_eq!(source_position("abc", 4), None);
    assert_eq!(source_position("", 1), None);
    assert_eq!(source_position("é", 1), None);
    assert_eq!(source_position("a😀b", 3), None);
    assert_eq!(source_position("a😀b", 5), Some((1, 3)));
}
