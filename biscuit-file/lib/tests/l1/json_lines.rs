//! `read_json_lines`: values, physical positions, and invalid records.

use biscuit_file::json_lines::{JsonLineRecord, JsonLinesError, read_json_lines};
use serde_json::json;

#[test]
fn each_line_is_one_value_in_order() {
    let rows = read_json_lines("{\"a\":1}\n[1,2]\n\"s\"\nnull\n").unwrap();

    assert_eq!(
        rows.values,
        vec![json!({"a": 1}), json!([1, 2]), json!("s"), json!(null)]
    );
    assert_eq!(
        rows.line_offsets,
        vec![
            JsonLineRecord { line: 1, offset: 0, text: 0..7 },
            JsonLineRecord { line: 2, offset: 8, text: 8..13 },
            JsonLineRecord { line: 3, offset: 14, text: 14..17 },
            JsonLineRecord { line: 4, offset: 18, text: 18..22 },
        ]
    );
}

#[test]
fn blank_and_whitespace_only_lines_are_skipped_but_counted() {
    let text = "\n1\n  \t \n\n2";
    let rows = read_json_lines(text).unwrap();

    assert_eq!(rows.values, vec![json!(1), json!(2)]);
    assert_eq!(rows.line_offsets[0].line, 2);
    assert_eq!(rows.line_offsets[0].offset, 1);
    assert_eq!(rows.line_offsets[1].line, 5);
    assert_eq!(rows.line_offsets[1].offset, text.len() - 1);
}

#[test]
fn crlf_line_endings_are_accepted_and_trimmed_from_the_record_text() {
    let text = "{\"a\":1}\r\n\r\n  [2] \r\n";
    let rows = read_json_lines(text).unwrap();

    assert_eq!(rows.values, vec![json!({"a": 1}), json!([2])]);
    assert_eq!(rows.line_offsets[0], JsonLineRecord { line: 1, offset: 0, text: 0..7 });
    assert_eq!(rows.line_offsets[1].line, 3);
    assert_eq!(rows.line_offsets[1].offset, 11);
    assert_eq!(&text[rows.line_offsets[1].text.clone()], "[2]");
}

#[test]
fn record_text_range_keeps_the_original_number_spelling() {
    let text = "1.50\n{\"n\": 9007199254740993}\n";
    let rows = read_json_lines(text).unwrap();

    assert_eq!(&text[rows.line_offsets[0].text.clone()], "1.50");
    assert_eq!(&text[rows.line_offsets[1].text.clone()], "{\"n\": 9007199254740993}");
}

#[test]
fn zero_byte_text_has_no_values() {
    let rows = read_json_lines("").unwrap();

    assert!(rows.values.is_empty());
    assert!(rows.line_offsets.is_empty());
}

#[test]
fn an_invalid_record_reports_its_line_and_byte_offset() {
    // `é` is two bytes, so a character count would put the error one byte
    // early.
    let text = "{\"ok\":1}\n\n{\"é\": tru}\n";
    let JsonLinesError::InvalidRecord { line, offset, .. } = read_json_lines(text).unwrap_err();

    // serde_json stops at the byte that breaks the `true` keyword.
    assert_eq!(line, 3);
    assert_eq!(offset, text.find("tru}").unwrap() + 3);
}

#[test]
fn an_unexpected_character_is_located_exactly() {
    let text = "1\n[\"ü\", @]\n";
    let JsonLinesError::InvalidRecord { line, offset, .. } = read_json_lines(text).unwrap_err();

    assert_eq!(line, 2);
    assert_eq!(offset, text.find('@').unwrap());
}

#[test]
fn two_values_on_one_line_are_invalid() {
    let text = "1 2\n";
    let JsonLinesError::InvalidRecord { line, offset, .. } = read_json_lines(text).unwrap_err();

    assert_eq!(line, 1);
    assert_eq!(offset, 2);
}
