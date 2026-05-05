//! OSC color response parsing and ANSI palette helpers.

use super::types::RgbValue;

/// Parse an OSC color response from the terminal.
///
/// Terminals respond to OSC 10/11/12 queries with:
/// - BEL-terminated: `\x1b]<code>;rgb:<rrrr>/<gggg>/<bbbb>\x07`
/// - ST-terminated: `\x1b]<code>;rgb:<rrrr>/<gggg>/<bbbb>\x1b\\`
///
/// The RGB values are 16-bit hex values (0000-ffff) that need to be
/// converted to 8-bit (0-255).
///
/// ## Arguments
///
/// * `response` - Raw bytes from terminal response
/// * `expected_code` - The OSC code we expect (10, 11, or 12)
///
/// ## Returns
///
/// `Some(RgbValue)` if parsing succeeded, `None` otherwise.
pub fn parse_osc_color_response(response: &[u8], expected_code: u8) -> Option<RgbValue> {
    let response_str = std::str::from_utf8(response).ok()?;

    // Find the end of the response (BEL or ST)
    let end_pos = response_str
        .find('\x07')
        .or_else(|| response_str.find("\x1b\\"))
        .unwrap_or(response_str.len());

    let content = &response_str[..end_pos];

    // Skip leading escape sequence if present
    let content = content.strip_prefix("\x1b]").unwrap_or(content);

    // Parse code and rgb values
    let parts: Vec<&str> = content.splitn(2, ';').collect();
    if parts.len() != 2 {
        return None;
    }

    // Verify the code matches
    let code: u8 = parts[0].parse().ok()?;
    if code != expected_code {
        return None;
    }

    // Parse rgb:<rrrr>/<gggg>/<bbbb>
    let rgb_part = parts[1].strip_prefix("rgb:")?;
    let rgb_parts: Vec<&str> = rgb_part.split('/').collect();
    if rgb_parts.len() != 3 {
        return None;
    }

    // Parse 16-bit hex values and convert to 8-bit
    let r16 = u16::from_str_radix(rgb_parts[0], 16).ok()?;
    let g16 = u16::from_str_radix(rgb_parts[1], 16).ok()?;
    let b16 = u16::from_str_radix(rgb_parts[2], 16).ok()?;

    Some(RgbValue::new(
        convert_16bit_to_8bit(r16),
        convert_16bit_to_8bit(g16),
        convert_16bit_to_8bit(b16),
    ))
}

/// Convert a 16-bit color component to 8-bit with proper rounding.
///
/// Uses the formula: `(val * 255 + 32767) / 65535`
/// This ensures 0xffff maps to 255, not 254.
#[inline]
pub(super) fn convert_16bit_to_8bit(val: u16) -> u8 {
    ((val as u32 * 255 + 32767) / 65535) as u8
}

/// Parse the `COLORFGBG` environment variable.
///
/// Format: `"fg_index;bg_index"` where indices are ANSI color numbers (0-15).
/// Some terminals use `"fg;bg;brightness"` format with an optional third value.
///
/// - code 10: foreground
/// - code 11: background
/// - code 12: cursor (typically same as foreground)
pub(super) fn parse_colorfgbg(value: &str, code: u8) -> Option<RgbValue> {
    let parts: Vec<&str> = value.split(';').collect();
    if parts.len() < 2 {
        return None;
    }

    let index = match code {
        10 | 12 => parts[0].parse::<u8>().ok()?,
        11 => parts.get(1).and_then(|s| s.parse::<u8>().ok())?,
        _ => return None,
    };

    ansi_index_to_rgb(index)
}

/// Convert ANSI color index (0-15) to RGB.
///
/// Uses the standard ANSI/VGA color palette:
/// - 0-7: Normal colors (black, red, green, yellow, blue, magenta, cyan, white)
/// - 8-15: Bright variants
pub(super) fn ansi_index_to_rgb(index: u8) -> Option<RgbValue> {
    match index {
        0 => Some(RgbValue::new(0, 0, 0)),        // Black
        1 => Some(RgbValue::new(205, 49, 49)),    // Red
        2 => Some(RgbValue::new(13, 188, 121)),   // Green
        3 => Some(RgbValue::new(229, 229, 16)),   // Yellow
        4 => Some(RgbValue::new(36, 114, 200)),   // Blue
        5 => Some(RgbValue::new(188, 63, 188)),   // Magenta
        6 => Some(RgbValue::new(17, 168, 205)),   // Cyan
        7 => Some(RgbValue::new(229, 229, 229)),  // White
        8 => Some(RgbValue::new(102, 102, 102)),  // Bright Black (Gray)
        9 => Some(RgbValue::new(241, 76, 76)),    // Bright Red
        10 => Some(RgbValue::new(35, 209, 139)),  // Bright Green
        11 => Some(RgbValue::new(245, 245, 67)),  // Bright Yellow
        12 => Some(RgbValue::new(59, 142, 234)),  // Bright Blue
        13 => Some(RgbValue::new(214, 112, 214)), // Bright Magenta
        14 => Some(RgbValue::new(41, 184, 219)),  // Bright Cyan
        15 => Some(RgbValue::new(255, 255, 255)), // Bright White
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_index_to_rgb_valid() {
        let black = ansi_index_to_rgb(0);
        assert!(black.is_some());
        assert_eq!(black.unwrap(), RgbValue::new(0, 0, 0));

        let white = ansi_index_to_rgb(15);
        assert!(white.is_some());
        assert_eq!(white.unwrap(), RgbValue::new(255, 255, 255));

        for i in 0..16 {
            assert!(
                ansi_index_to_rgb(i).is_some(),
                "Index {} should be valid",
                i
            );
        }
    }

    #[test]
    fn test_ansi_index_to_rgb_invalid() {
        assert!(ansi_index_to_rgb(16).is_none());
        assert!(ansi_index_to_rgb(255).is_none());
    }

    #[test]
    fn test_parse_colorfgbg_valid_dark() {
        let bg = parse_colorfgbg("15;0", 11);
        assert!(bg.is_some());
        assert_eq!(bg.unwrap(), RgbValue::new(0, 0, 0));

        let fg = parse_colorfgbg("15;0", 10);
        assert!(fg.is_some());
        assert_eq!(fg.unwrap(), RgbValue::new(255, 255, 255));
    }

    #[test]
    fn test_parse_colorfgbg_valid_light() {
        let bg = parse_colorfgbg("0;15", 11);
        assert!(bg.is_some());
        assert_eq!(bg.unwrap(), RgbValue::new(255, 255, 255));

        let fg = parse_colorfgbg("0;15", 10);
        assert!(fg.is_some());
        assert_eq!(fg.unwrap(), RgbValue::new(0, 0, 0));
    }

    #[test]
    fn test_parse_colorfgbg_cursor_uses_fg() {
        let cursor = parse_colorfgbg("15;0", 12);
        assert!(cursor.is_some());
        assert_eq!(cursor.unwrap(), RgbValue::new(255, 255, 255));
    }

    #[test]
    fn test_parse_colorfgbg_with_brightness() {
        let bg = parse_colorfgbg("7;0;1", 11);
        assert!(bg.is_some());
        assert_eq!(bg.unwrap(), RgbValue::new(0, 0, 0));
    }

    #[test]
    fn test_parse_colorfgbg_invalid() {
        assert!(parse_colorfgbg("", 11).is_none());
        assert!(parse_colorfgbg("abc", 11).is_none());
        assert!(parse_colorfgbg("15", 11).is_none());
        assert!(parse_colorfgbg("15;0", 99).is_none());
    }

    #[test]
    fn test_parse_colorfgbg_out_of_range_index() {
        assert!(parse_colorfgbg("20;0", 10).is_none());
        assert!(parse_colorfgbg("15;20", 11).is_none());
    }

    #[test]
    fn test_parse_osc_color_response_bel_terminated() {
        let response = b"\x1b]11;rgb:ffff/ffff/ffff\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 255);
        assert_eq!(rgb.b, 255);
    }

    #[test]
    fn test_parse_osc_color_response_st_terminated() {
        let response = b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 255);
        assert_eq!(rgb.b, 255);
    }

    #[test]
    fn test_parse_osc_color_response_black() {
        let response = b"\x1b]11;rgb:0000/0000/0000\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 0);
        assert_eq!(rgb.g, 0);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_parse_osc_color_response_various_codes() {
        let response = b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07";
        let result = parse_osc_color_response(response, 10);
        assert!(result.is_some());

        let response = b"\x1b]12;rgb:00ff/ff00/0000\x07";
        let result = parse_osc_color_response(response, 12);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_wrong_code() {
        let response = b"\x1b]11;rgb:ffff/ffff/ffff\x07";
        let result = parse_osc_color_response(response, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_osc_color_response_invalid_format() {
        let response = b"\x1b]11;ffff/ffff/ffff\x07";
        assert!(parse_osc_color_response(response, 11).is_none());

        let response = b"\x1b]11;rgb:ffff/ffff\x07";
        assert!(parse_osc_color_response(response, 11).is_none());

        let response = b"\x1b]11;rgb:gggg/ffff/ffff\x07";
        assert!(parse_osc_color_response(response, 11).is_none());

        assert!(parse_osc_color_response(b"", 11).is_none());
    }

    #[test]
    fn test_convert_16bit_to_8bit() {
        assert_eq!(convert_16bit_to_8bit(0), 0);
        assert_eq!(convert_16bit_to_8bit(0xffff), 255);
        assert_eq!(convert_16bit_to_8bit(0x8000), 128);
        assert_eq!(convert_16bit_to_8bit(0x0101), 1);
        assert_eq!(convert_16bit_to_8bit(0xfefe), 254);
    }

    #[test]
    fn test_parse_osc_color_response_mid_gray() {
        let response = b"\x1b]11;rgb:8080/8080/8080\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 128);
        assert_eq!(rgb.g, 128);
        assert_eq!(rgb.b, 128);
    }

    #[test]
    fn test_parse_osc_color_response_with_trailing_data() {
        let response = b"\x1b]11;rgb:ffff/0000/0000\x07extra data";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 0);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_parse_osc_color_response_lowercase_hex() {
        let response = b"\x1b]11;rgb:abcd/ef01/2345\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_uppercase_hex() {
        let response = b"\x1b]11;rgb:ABCD/EF01/2345\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_mixed_case_hex() {
        let response = b"\x1b]11;rgb:AbCd/eF01/23aB\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_short_hex() {
        let response = b"\x1b]11;rgb:ff/ff/ff\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert!(rgb.r <= 1);
    }

    #[test]
    fn test_convert_16bit_to_8bit_full_range() {
        for i in 0u8..=255 {
            let val16 = (i as u32 * 65535 / 255) as u16;
            let result = convert_16bit_to_8bit(val16);
            assert!(
                result.abs_diff(i) <= 1,
                "16-bit {} should map to ~{}, got {}",
                val16,
                i,
                result
            );
        }
    }
}
