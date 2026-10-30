//! The shared foundations through the crate's public API: spans index the
//! exact supplied text (CRLF and Windows paths included), the numeric model's
//! boundaries, and the literal-token codec agreeing with the `{{ }}` scan.

use serde_json::json;
use simplified_schema::expression::scan::scan_plain;
use simplified_schema::numeric::{
    NumberTextError, NumericAdmissionError, admit_integer, admit_json_text_numbers, admit_number_spelling,
    load_admitted_yaml,
};
use simplified_schema::pending::{TokenError, decode_leaf, decode_literal_tokens, encode, holds_pending_syntax};
use simplified_schema::{SourceSpan, Spanned, line_col_of_offset, line_of_offset};

#[test]
fn spans_are_byte_offsets_into_crlf_text_with_windows_paths() {
    let source = "path: C:\\Users\\ken\\notes.md\r\nlong: \\\\?\\C:\\very\\long\r\nlast: x";
    let long = source.find("\\\\?\\C:").expect("the verbatim path is present");
    let spanned = Spanned::new("long", long..long + 4);
    let span: SourceSpan = spanned.span.clone();

    assert_eq!(&source[span.clone()], "\\\\?\\");
    // The CRLF counts as one line break and the column is a byte column.
    assert_eq!(line_col_of_offset(source, span.start), (2, 7));
    assert_eq!(line_of_offset(source, span.start), 2);
    let last = source.find("last").expect("the last line is present");
    assert_eq!(line_col_of_offset(source, last), (3, 1));
    // An offset between `\r` and `\n` is still on the line the CRLF ends.
    let cr = source.find('\r').expect("a CR is present");
    assert_eq!(line_col_of_offset(source, cr + 1), (1, cr + 2));
}

#[test]
fn integer_admission_boundary_is_two_to_the_fifty_third() {
    assert_eq!(admit_integer(1 << 53), Ok(9_007_199_254_740_992.0));
    assert_eq!(admit_integer((1 << 53) + 1), Err(NumericAdmissionError::Inexact));
    assert_eq!(
        admit_number_spelling("9007199254740993.0"),
        Err(NumberTextError::Admission(NumericAdmissionError::Inexact))
    );
    assert_eq!(admit_number_spelling("0.1"), Ok(0.1));
    assert_eq!(admit_number_spelling("pear"), Err(NumberTextError::NotANumber));
    assert_eq!(admit_number_spelling("1e400"), Err(NumberTextError::Admission(NumericAdmissionError::NotFinite)));
}

#[test]
fn loaders_reject_inadmissible_numbers_with_a_location() {
    let error = load_admitted_yaml("a:\n  b: 9007199254740993\n").unwrap_err().to_string();
    assert!(error.contains("a.b"), "{error}");
    assert!(error.contains("cannot be represented exactly"), "{error}");
    assert!(load_admitted_yaml("a: 9007199254740992\n").is_ok());

    let error = admit_json_text_numbers("{\r\n  \"n\": 9007199254740993\r\n}").unwrap_err();
    assert_eq!(error.pointer, "/n");
    assert_eq!((error.line, error.column), (2, 8));
    assert!(admit_json_text_numbers("{\"n\": 1.5}").is_ok());
}

#[test]
fn a_whole_leaf_token_is_data_and_never_pending() {
    let token = encode("{{ area }} $(pwd)");
    assert_eq!(decode_leaf(&token), Some(Ok("{{ area }} $(pwd)".to_string())));
    assert!(!holds_pending_syntax(&token));
    assert!(holds_pending_syntax("{{ area }}"));
    assert!(holds_pending_syntax("$(pwd)"));
    assert!(!holds_pending_syntax("plain"));

    // A token inside other text is malformed and still pending.
    let embedded = format!("x {token}");
    assert_eq!(decode_leaf(&embedded), Some(Err(TokenError::Embedded)));
    assert!(holds_pending_syntax(&embedded));

    // An escaped spelling is text: the scanner finds no token in it.
    assert_eq!(decode_leaf("{{{ {{!data:v1:YQ}} }}}"), None);
    assert!(scan_plain("{{{ {{!data:v1:YQ}} }}}").tokens.is_empty());

    let decoded = decode_literal_tokens(&json!({"a": [token.clone()], "b": 1})).unwrap();
    assert_eq!(decoded, json!({"a": ["{{ area }} $(pwd)"], "b": 1}));
}
