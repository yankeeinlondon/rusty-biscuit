use crate::args::CliFill;
use darkmatter::markdown::highlighting::ThemePair;
use renderable::layout::Length;

/// Parses and validates list indentation width.
pub fn parse_indent_size(s: &str) -> Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;

    match value {
        2 | 4 | 8 => Ok(value),
        _ => Err("indent must be one of: 2, 4, 8".to_string()),
    }
}

/// Parses `--fixed-width`, accepting practical Markdown wrap columns from 1 to 1000.
///
/// The upper bound rejects accidental huge values while still allowing any
/// realistic terminal/editor width.
pub fn parse_fixed_width(s: &str) -> Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| format!("'{s}' is not a valid positive integer"))?;

    if (1..=1000).contains(&value) {
        Ok(value)
    } else {
        Err("--fixed-width must be between 1 and 1000".to_string())
    }
}

/// Parses a theme name string into ThemePair.
pub fn parse_theme_name(s: &str) -> Result<ThemePair, String> {
    ThemePair::try_from(s).map_err(|e| e.to_string())
}

/// Parses a [`CliFill`] string.
///
/// Grammar: `full`, `pad=<n|n%>`, `indent=<n|n%>`, `max=<n|n%>`, `explicit=<n|n%>`
pub fn parse_cli_fill(s: &str) -> Result<CliFill, String> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("full") {
        return Ok(CliFill::Full);
    }

    let (kind, rest) = s
        .split_once('=')
        .ok_or_else(|| format!("fill must be 'full' or 'kind=value', got '{s}'"))?;

    let kind = kind.trim().to_ascii_lowercase();
    let rest = rest.trim();

    if rest.is_empty() {
        return Err(format!("fill value missing after '=' in '{s}'"));
    }

    let length = parse_cli_length(rest)?;

    match kind.as_str() {
        "pad" => Ok(CliFill::Pad(length)),
        "indent" => Ok(CliFill::Indent(length)),
        "max" => Ok(CliFill::Max(length)),
        "explicit" => Ok(CliFill::Explicit(length)),
        _ => Err(format!(
            "unknown fill kind '{kind}', expected pad/indent/max/explicit"
        )),
    }
}

/// Parses a length string (`n` or `n%`) into a renderable [`Length`].
pub fn parse_cli_length(s: &str) -> Result<Length, String> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('%') {
        let p: f32 = num
            .parse()
            .map_err(|_| format!("'{s}' is not a valid percentage"))?;
        if !(0.0..=100.0).contains(&p) {
            return Err(format!("percentage must be 0-100, got {p}"));
        }
        Ok(Length::Percent(p))
    } else {
        let n: u16 = s
            .parse()
            .map_err(|_| format!("'{s}' is not a valid positive integer"))?;
        Ok(Length::ch(u32::from(n)))
    }
}

/// Parses a boolean string (`true`/`false`/`1`/`0`/`yes`/`no`).
pub fn parse_bool_str(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "y" => Ok(true),
        "false" | "0" | "no" | "off" | "n" => Ok(false),
        _ => Err(format!("expected true/false, got '{s}'")),
    }
}

/// Parses `--max-width`, rejecting `0`.
pub fn parse_max_width(s: &str) -> Result<u16, String> {
    let value = s
        .parse::<u16>()
        .map_err(|_| format!("'{s}' is not a valid positive integer"))?;
    if value == 0 {
        Err("--max-width must be greater than 0".to_string())
    } else {
        Ok(value)
    }
}

/// Reject `--width` outright. Width is derived from the captured `Terminal`
/// and the configured `--max-width`; a separate `--width` flag would shadow
/// those for no reason. Callers should use `--max-width` for content-width
/// control.
pub fn reject_width_flag(_s: &str) -> Result<u16, String> {
    Err(
        "--width is not supported: page width is captured from the terminal; \
         use --max-width to cap the content box."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::color::{Color, Tailwind};
    use renderable::style::PaintColor;

    #[test]
    fn parse_indent_size_accepts_valid_values() {
        assert_eq!(parse_indent_size("2"), Ok(2));
        assert_eq!(parse_indent_size("4"), Ok(4));
        assert_eq!(parse_indent_size("8"), Ok(8));
    }

    #[test]
    fn parse_indent_size_rejects_invalid() {
        assert!(parse_indent_size("3").is_err());
        assert!(parse_indent_size("abc").is_err());
    }

    #[test]
    fn parse_fixed_width_accepts_valid_values() {
        assert_eq!(parse_fixed_width("1"), Ok(1));
        assert_eq!(parse_fixed_width("80"), Ok(80));
        assert_eq!(parse_fixed_width("1000"), Ok(1000));
    }

    #[test]
    fn parse_fixed_width_rejects_invalid_values() {
        assert!(parse_fixed_width("0").is_err());
        assert!(parse_fixed_width("1001").is_err());
        assert!(parse_fixed_width("abc").is_err());
        assert!(parse_fixed_width("-1").is_err());
    }

    #[test]
    fn parse_bool_str_accepts_truthy_values() {
        for val in ["true", "1", "yes", "on", "y"] {
            assert!(parse_bool_str(val).unwrap(), "value: {val}");
        }
    }

    #[test]
    fn parse_bool_str_accepts_falsy_values() {
        for val in ["false", "0", "no", "off", "n"] {
            assert!(!parse_bool_str(val).unwrap(), "value: {val}");
        }
    }

    #[test]
    fn parse_bool_str_rejects_invalid() {
        assert!(parse_bool_str("maybe").is_err());
        assert!(parse_bool_str("").is_err());
    }

    #[test]
    fn parse_max_width_accepts_positive() {
        assert_eq!(parse_max_width("80").unwrap(), 80);
        assert_eq!(parse_max_width("1").unwrap(), 1);
    }

    #[test]
    fn parse_max_width_rejects_zero() {
        assert!(parse_max_width("0").is_err());
    }

    #[test]
    fn parse_max_width_rejects_negative() {
        assert!(parse_max_width("-1").is_err());
    }

    #[test]
    fn reject_width_flag_always_errors() {
        assert!(reject_width_flag("80").is_err());
        assert!(reject_width_flag("0").is_err());
    }

    #[test]
    fn parse_cli_fill_full() {
        assert_eq!(parse_cli_fill("full").unwrap(), CliFill::Full);
        assert_eq!(parse_cli_fill("FULL").unwrap(), CliFill::Full);
    }

    #[test]
    fn parse_cli_fill_pad_fixed() {
        assert_eq!(
            parse_cli_fill("pad=4").unwrap(),
            CliFill::Pad(Length::ch(4))
        );
    }

    #[test]
    fn parse_cli_fill_pad_percent() {
        assert_eq!(
            parse_cli_fill("pad=10%").unwrap(),
            CliFill::Pad(Length::Percent(10.0))
        );
    }

    #[test]
    fn parse_cli_fill_indent_max_explicit() {
        assert_eq!(
            parse_cli_fill("indent=2").unwrap(),
            CliFill::Indent(Length::ch(2))
        );
        assert_eq!(
            parse_cli_fill("max=40").unwrap(),
            CliFill::Max(Length::ch(40))
        );
        assert_eq!(
            parse_cli_fill("explicit=60").unwrap(),
            CliFill::Explicit(Length::ch(60))
        );
    }

    #[test]
    fn parse_cli_fill_rejects_unknown_kind() {
        assert!(parse_cli_fill("unknown=4").is_err());
    }

    #[test]
    fn parse_cli_fill_rejects_percent_over_100() {
        assert!(parse_cli_fill("pad=150%").is_err());
    }

    #[test]
    fn parse_cli_fill_rejects_negative() {
        assert!(parse_cli_fill("pad=-1").is_err());
    }

    #[test]
    fn parse_cli_fill_rejects_malformed() {
        assert!(parse_cli_fill("pad").is_err());
        assert!(parse_cli_fill("=").is_err());
    }

    #[test]
    fn parse_cli_length_fixed() {
        assert_eq!(parse_cli_length("80").unwrap(), Length::ch(80));
    }

    #[test]
    fn parse_cli_length_percent() {
        assert_eq!(parse_cli_length("50%").unwrap(), Length::Percent(50.0));
    }

    #[test]
    fn parse_cli_length_rejects_out_of_range_percent() {
        assert!(parse_cli_length("150%").is_err());
        assert!(parse_cli_length("-10%").is_err());
    }

    #[test]
    fn page_bg_color_hex_six() {
        let c = PaintColor::from_css_str("#1e1e23").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 0x1e);
                assert_eq!(rgb.green(), 0x1e);
                assert_eq!(rgb.blue(), 0x23);
            }
            other => panic!("expected RGB color, got {other:?}"),
        }
    }

    #[test]
    fn page_bg_color_hex_three() {
        let c = PaintColor::from_css_str("#abc").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 0xaa);
                assert_eq!(rgb.green(), 0xbb);
                assert_eq!(rgb.blue(), 0xcc);
            }
            other => panic!("expected RGB color, got {other:?}"),
        }
    }

    #[test]
    fn page_bg_color_rgb_triple() {
        let c = PaintColor::from_css_str("30,30,35").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 30);
                assert_eq!(rgb.green(), 30);
                assert_eq!(rgb.blue(), 35);
            }
            other => panic!("expected RGB color, got {other:?}"),
        }
    }

    #[test]
    fn page_bg_color_tailwind() {
        let c = PaintColor::from_css_str("red-500").unwrap();
        assert!(matches!(c.color, Color::Tailwind(Tailwind::Red500)));
    }

    #[test]
    fn page_bg_color_special_keywords() {
        assert!(matches!(
            PaintColor::from_css_str("transparent").unwrap().color,
            Color::Tailwind(Tailwind::Transparent)
        ));
        assert!(matches!(
            PaintColor::from_css_str("currentColor").unwrap().color,
            Color::Tailwind(Tailwind::Current)
        ));
        assert!(matches!(
            PaintColor::from_css_str("inherit").unwrap().color,
            Color::Tailwind(Tailwind::Inherit)
        ));
    }

    #[test]
    fn page_bg_color_rejects_invalid() {
        assert!(PaintColor::from_css_str("").is_err());
        assert!(PaintColor::from_css_str("not-a-color").is_err());
        assert!(PaintColor::from_css_str("#zzz").is_err());
        assert!(PaintColor::from_css_str("256,0,0").is_err());
        assert!(PaintColor::from_css_str("1,2").is_err());
    }
}
