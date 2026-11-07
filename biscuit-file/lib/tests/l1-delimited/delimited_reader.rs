//! `read_delimited` over CSV and TSV: decoded cells, raw source positions,
//! and strict rejection.
//!
//! Most cases are written with `,` and run once per delimiter by replacing
//! every `,` with the delimiter under test; quoting rules and offsets are the
//! same for both.

use biscuit_file::delimited::{DelimitedData, DelimitedError, DelimitedRecord, read_delimited};

const DELIMITERS: [u8; 2] = *b",\t";

fn read_as(text: &str, delimiter: u8) -> Result<DelimitedData, DelimitedError> {
    let text = text.replace(',', &char::from(delimiter).to_string());
    read_delimited(&text, delimiter)
}

fn texts(record: &DelimitedRecord) -> Vec<&str> {
    record.fields.iter().map(|field| field.text.as_str()).collect()
}

fn offsets(record: &DelimitedRecord) -> Vec<usize> {
    record.fields.iter().map(|field| field.offset).collect()
}

#[test]
fn doubled_quote_fields_report_raw_source_starts() {
    for delimiter in DELIMITERS {
        let data = read_as("a,\"x\"\"y,z\",q\n", delimiter).unwrap();
        let header = data.headers.unwrap();

        let separator = char::from(delimiter);
        assert_eq!(texts(&header), ["a", &format!("x\"y{separator}z"), "q"]);
        assert_eq!(offsets(&header), [0, 2, 11]);
        assert_eq!((header.line, header.offset), (1, 0));
        assert!(data.rows.is_empty());
    }
}

#[test]
fn quoted_newline_keeps_later_offsets_and_lines_physical() {
    for delimiter in DELIMITERS {
        let data = read_as("h1,h2,h3\n\"a\nb\",c,d\ne,f,g\n", delimiter).unwrap();

        let first = &data.rows[0];
        assert_eq!(texts(first), ["a\nb", "c", "d"]);
        assert_eq!(offsets(first), [9, 15, 17]);
        assert_eq!((first.line, first.offset), (2, 9));

        let second = &data.rows[1];
        assert_eq!(texts(second), ["e", "f", "g"]);
        assert_eq!(offsets(second), [19, 21, 23]);
        assert_eq!((second.line, second.offset), (4, 19));
    }
}

#[test]
fn crlf_terminates_records_without_leaking_into_cells() {
    for delimiter in DELIMITERS {
        let data = read_as("a,b\r\n1,\"2\"\r\n3,4", delimiter).unwrap();

        assert_eq!(texts(&data.headers.unwrap()), ["a", "b"]);
        assert_eq!(texts(&data.rows[0]), ["1", "2"]);
        assert_eq!(offsets(&data.rows[0]), [5, 7]);
        assert_eq!(data.rows[0].line, 2);
        assert_eq!(texts(&data.rows[1]), ["3", "4"]);
        assert_eq!((data.rows[1].line, data.rows[1].offset), (3, 12));
    }
}

#[test]
fn blank_lines_are_skipped_but_counted() {
    for delimiter in DELIMITERS {
        let data = read_as("a,b\n\n1,2\n\r\n3,4\n\n", delimiter).unwrap();

        assert_eq!(data.rows.len(), 2);
        assert_eq!((data.rows[0].line, data.rows[0].offset), (3, 5));
        assert_eq!((data.rows[1].line, data.rows[1].offset), (5, 11));
        assert_eq!(texts(&data.rows[1]), ["3", "4"]);
    }
}

#[test]
fn a_header_after_leading_blank_lines_reports_its_real_line() {
    for delimiter in DELIMITERS {
        let data = read_as("\n\r\na,b\n1,2", delimiter).unwrap();
        let header = data.headers.unwrap();

        assert_eq!((header.line, header.offset), (3, 3));
        assert_eq!((data.rows[0].line, data.rows[0].offset), (4, 7));
    }
}

#[test]
fn a_delimiter_only_row_is_a_record_of_empty_cells() {
    for delimiter in DELIMITERS {
        let data = read_as("a,b,c\n,,\n", delimiter).unwrap();

        assert_eq!(texts(&data.rows[0]), ["", "", ""]);
        assert_eq!(offsets(&data.rows[0]), [6, 7, 8]);
    }
}

#[test]
fn cell_and_header_text_is_never_trimmed() {
    for delimiter in DELIMITERS {
        let data = read_as(" h1 ,h2\n x ,\" y \"\n", delimiter).unwrap();

        assert_eq!(texts(&data.headers.unwrap()), [" h1 ", "h2"]);
        assert_eq!(texts(&data.rows[0]), [" x ", " y "]);
    }
}

#[test]
fn a_header_only_text_has_no_rows() {
    for delimiter in DELIMITERS {
        for text in ["a,b\n", "a,b"] {
            let data = read_as(text, delimiter).unwrap();

            assert_eq!(texts(&data.headers.unwrap()), ["a", "b"]);
            assert!(data.rows.is_empty(), "{text:?}");
        }
    }
}

#[test]
fn empty_or_blank_text_has_no_header_and_no_rows() {
    for delimiter in DELIMITERS {
        for text in ["", "\n\r\n"] {
            let data = read_as(text, delimiter).unwrap();

            assert_eq!(data, DelimitedData { headers: None, rows: Vec::new() }, "{text:?}");
        }
    }
}

#[test]
fn a_row_wider_than_the_header_is_rejected_at_its_start() {
    for delimiter in DELIMITERS {
        let error = read_as("a,b\n1,2,3\n", delimiter).unwrap_err();

        assert_eq!(
            error,
            DelimitedError::RaggedRow { line: 2, offset: 4, expected: 2, found: 3 }
        );
    }
}

#[test]
fn a_row_narrower_than_the_header_is_rejected_at_its_start() {
    for delimiter in DELIMITERS {
        let error = read_as("a,b\n1,2\n\"x\ny\"\n", delimiter).unwrap_err();

        assert_eq!(
            error,
            DelimitedError::RaggedRow { line: 3, offset: 8, expected: 2, found: 1 }
        );
    }
}

#[test]
fn an_unterminated_quote_is_rejected_at_its_opening_quote() {
    for delimiter in DELIMITERS {
        let error = read_as("a,b\n1,\"oops\n2,3\n", delimiter).unwrap_err();

        assert_eq!(error, DelimitedError::UnterminatedQuote { line: 2, offset: 6 });
    }
}

#[test]
fn a_quote_inside_an_unquoted_field_is_rejected() {
    for delimiter in DELIMITERS {
        let error = read_as("a,b\"c,d", delimiter).unwrap_err();

        assert_eq!(error, DelimitedError::QuoteInUnquotedField { line: 1, offset: 3 });
    }
}

#[test]
fn text_after_a_closing_quote_is_rejected() {
    for delimiter in DELIMITERS {
        assert_eq!(
            read_as("\"a\"x,b", delimiter).unwrap_err(),
            DelimitedError::TextAfterClosingQuote { line: 1, offset: 3 }
        );
        assert_eq!(
            read_as("a,b\n\"x\" ,y\n", delimiter).unwrap_err(),
            DelimitedError::TextAfterClosingQuote { line: 2, offset: 7 }
        );
    }
}

#[test]
fn a_comma_is_plain_text_in_tsv() {
    let data = read_delimited("a\tb\nx,y\t\"z\tw\"\n", b'\t').unwrap();

    assert_eq!(texts(&data.rows[0]), ["x,y", "z\tw"]);
    assert_eq!(offsets(&data.rows[0]), [4, 8]);
}

#[test]
fn a_delimiter_that_conflicts_with_quoting_or_lines_is_rejected() {
    for delimiter in [b'"', b'\n', b'\r', 0xC3] {
        assert_eq!(
            read_delimited("a", delimiter).unwrap_err(),
            DelimitedError::InvalidDelimiter(delimiter)
        );
    }
}
