//! Helpers for asserting on captured pane geometry in Level-2 image
//! and layout tests.
//!
//! These helpers translate captured `frame.plain` output into row
//! indices so tests can assert on the *user-visible* placement of text
//! and image-spacer regions rather than just on substring presence.

#![allow(dead_code)]

/// Returns the 0-based row index of the first line in `plain` that
/// contains `needle`, or `None` if no such line exists.
pub fn find_row_of(plain: &str, needle: &str) -> Option<usize> {
    plain.lines().position(|line| line.contains(needle))
}

/// Returns the number of non-empty lines in `plain`. Used to compare
/// pane-row counts before/after image rendering.
pub fn non_empty_row_count(plain: &str) -> usize {
    plain.lines().filter(|l| !l.trim().is_empty()).count()
}

/// Parses `image height: 12.81 raw -> ceil=13 floor=12` lines from `bt
/// image --debug` debug output.
///
/// Returns `(raw, ceil, floor)` triple when the line is present.
pub fn parse_debug_image_height(plain: &str) -> Option<(f32, u32, u32)> {
    let line = plain.lines().find(|l| l.contains("image height:"))?;
    let raw_part = line.split("image height:").nth(1)?;
    // raw_part looks like " 12.81 raw -> ceil=13 floor=12"
    let mut tokens = raw_part.split_whitespace();
    let raw: f32 = tokens.next()?.parse().ok()?;
    let mut ceil = None;
    let mut floor = None;
    for tok in tokens {
        if let Some(rest) = tok.strip_prefix("ceil=") {
            ceil = rest.parse().ok();
        } else if let Some(rest) = tok.strip_prefix("floor=") {
            floor = rest.parse().ok();
        }
    }
    Some((raw, ceil?, floor?))
}

/// Parses `cursor BEFORE: row=R col=C` from debug output.
pub fn parse_debug_cursor_before(plain: &str) -> Option<(u32, u32)> {
    let line = plain.lines().find(|l| l.contains("cursor BEFORE:"))?;
    let row = parse_kv(line, "row=")?;
    let col = parse_kv(line, "col=")?;
    Some((row, col))
}

/// Parses `cursor rows: N (used for CUD)` from debug output. This is the
/// renderer's chosen row advance count (ceil for most terminals, floor
/// for Warp).
pub fn parse_debug_cursor_rows(plain: &str) -> Option<u32> {
    let line = plain
        .lines()
        .find(|l| l.contains("cursor rows:") && l.contains("(used for CUD)"))?;
    let after = line.split("cursor rows:").nth(1)?;
    let first_tok = after.split_whitespace().next()?;
    first_tok.parse().ok()
}

/// Parses `image width: <N> cells` lines from `bt pie-chart --debug`
/// (or any other diagram subcommand that supports `--debug`).
///
/// Returns the integer cell count when the line is present.
pub fn parse_debug_image_width(plain: &str) -> Option<u32> {
    let line = plain.lines().find(|l| l.contains("image width:"))?;
    let after = line.split("image width:").nth(1)?;
    let first_tok = after.split_whitespace().next()?;
    first_tok.parse().ok()
}

/// Extracts the `c=N` parameter from a Kitty APC graphics payload.
///
/// The payload is the comma-separated parameter section of an APC
/// frame — typically `a=T,f=100,c=40,r=20;<base64>`. This helper splits
/// on `,`, finds the `c=` token, and parses the integer column count.
///
/// Returns `None` when the payload contains no `c=` parameter or the
/// value is not a valid integer.
pub fn extract_kitty_apc_columns(payload: &str) -> Option<u32> {
    let params = payload.split(';').next()?;
    for tok in params.split(',') {
        if let Some(rest) = tok.strip_prefix("c=") {
            return rest.parse().ok();
        }
    }
    None
}

fn parse_kv(line: &str, key: &str) -> Option<u32> {
    let after = line.split(key).nth(1)?;
    let mut digits = String::new();
    for ch in after.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_image_height_line() {
        let plain = "noise\nimage height: 12.81 raw -> ceil=13 floor=12\nmore\n";
        let (raw, ceil, floor) = parse_debug_image_height(plain).unwrap();
        assert!((raw - 12.81).abs() < 0.01);
        assert_eq!(ceil, 13);
        assert_eq!(floor, 12);
    }

    #[test]
    fn parses_cursor_before_line() {
        let plain = "x\ncursor BEFORE: row=22 col=1\ny\n";
        assert_eq!(parse_debug_cursor_before(plain), Some((22, 1)));
    }

    #[test]
    fn parses_cursor_rows() {
        let plain = "x\ncursor rows:  13 (used for CUD)\ny\n";
        assert_eq!(parse_debug_cursor_rows(plain), Some(13));
    }

    #[test]
    fn parses_image_width_line() {
        let plain = "noise\n--- mermaid debug ---\nimage width: 40 cells\nterm width: 80 cells\n";
        assert_eq!(parse_debug_image_width(plain), Some(40));
    }

    #[test]
    fn parses_image_width_returns_none_when_absent() {
        let plain = "no debug here\n";
        assert_eq!(parse_debug_image_width(plain), None);
    }

    #[test]
    fn extracts_kitty_apc_c_param() {
        let payload = "a=T,f=100,c=40,r=20;BASE64DATA";
        assert_eq!(extract_kitty_apc_columns(payload), Some(40));
    }

    #[test]
    fn extracts_kitty_apc_c_param_first_position() {
        let payload = "c=25,a=T,f=100;BASE64DATA";
        assert_eq!(extract_kitty_apc_columns(payload), Some(25));
    }

    #[test]
    fn extracts_kitty_apc_c_returns_none_without_param() {
        let payload = "a=T,f=100,r=20;BASE64DATA";
        assert_eq!(extract_kitty_apc_columns(payload), None);
    }

    #[test]
    fn extracts_kitty_apc_c_returns_none_for_garbage_value() {
        let payload = "a=T,c=notanumber;PAYLOAD";
        assert_eq!(extract_kitty_apc_columns(payload), None);
    }

    #[test]
    fn finds_row_of() {
        let plain = "row0\nrow1\nrow2\nNEEDLE here\nrow4\n";
        assert_eq!(find_row_of(plain, "NEEDLE"), Some(3));
        assert_eq!(find_row_of(plain, "absent"), None);
    }
}
