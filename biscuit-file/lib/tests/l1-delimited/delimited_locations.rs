//! CSV/TSV field offsets read back through `source_position`: each cell's raw
//! offset gives the line and character column an editor shows for it.

use biscuit_file::delimited::read_delimited;
use biscuit_file::source_position;

/// `(line, column)` of row `row`, field `field`, asserting the source there
/// starts with `token`.
fn cell_position(text: &str, delimiter: u8, row: usize, field: usize, token: &str) -> (usize, usize) {
    let data = read_delimited(text, delimiter).unwrap();
    let offset = data.rows[row].fields[field].offset;
    assert!(text[offset..].starts_with(token), "{:?} does not start with {token:?}", &text[offset..]);
    source_position(text, offset).unwrap()
}

#[test]
fn quoting_traps_before_the_target_cell_keep_its_raw_column() {
    // Embedded delimiter, doubled quote, and a quoted newline all precede
    // the bad cell; the quoted newline pushes the next row to line 4.
    let text = "name,quote,port\n\"a, inc\",\"say \"\"hi\"\"\",80\n\"multi\nline\",x,bad\n";
    assert_eq!(cell_position(text, b',', 0, 2, "80"), (2, 23));
    assert_eq!(cell_position(text, b',', 1, 0, "\"multi"), (3, 1));
    assert_eq!(cell_position(text, b',', 1, 2, "bad"), (4, 9));
}

#[test]
fn a_quoted_cell_starts_at_its_opening_quote() {
    let text = "a,b\n1,\"two\"\n";
    assert_eq!(cell_position(text, b',', 0, 1, "\"two\""), (2, 3));
}

#[test]
fn multibyte_cells_before_the_target_count_characters() {
    let text = "h1,h2\n日本,\"9\"\n";
    assert_eq!(cell_position(text, b',', 0, 1, "\"9\""), (2, 4));
}

#[test]
fn tsv_with_a_quoted_tab_keeps_the_raw_column() {
    let text = "a\tb\tc\n\"x\ty\"\té\tbad\n";
    assert_eq!(cell_position(text, b'\t', 0, 2, "bad"), (2, 9));
}

#[test]
fn crlf_twin_gives_the_same_line_and_column_as_lf() {
    let lf = "a,b\n\"q\nr\",é\n\n3,bad\n";
    let crlf = lf.replace('\n', "\r\n");
    for (row, field, token) in [(0, 1, "é"), (1, 1, "bad")] {
        assert_eq!(cell_position(lf, b',', row, field, token), cell_position(&crlf, b',', row, field, token));
    }
    assert_eq!(cell_position(lf, b',', 1, 1, "bad"), (5, 3));
}
