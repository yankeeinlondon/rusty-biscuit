use super::*;
use crate::terminal::Terminal;
use crate::utils::layout::RenderableWrapper;

/// Test that special Tailwind values return None
#[test]
fn tailwind_special_values_return_none() {
    assert!(Tailwind::Inherit.to_hdr_color().is_none());
    assert!(Tailwind::Current.to_hdr_color().is_none());
    assert!(Tailwind::Transparent.to_hdr_color().is_none());
}

/// Test that special Tailwind values return correct CSS values
#[test]
fn tailwind_special_css_vars() {
    assert_eq!(Tailwind::Inherit.css_var(), "inherit");
    assert_eq!(Tailwind::Current.css_var(), "currentColor");
    assert_eq!(Tailwind::Transparent.css_var(), "transparent");
}

/// Test that special Tailwind values return None for hex
#[test]
fn tailwind_special_hex_values() {
    assert!(Tailwind::Inherit.hex().is_none());
    assert!(Tailwind::Current.hex().is_none());
    assert!(Tailwind::Transparent.hex().is_none());
}

/// Test black and white basic values
#[test]
fn tailwind_black_white() {
    let black = Tailwind::Black.to_hdr_color().unwrap();
    assert_eq!(black.red(), 0);
    assert_eq!(black.green(), 0);
    assert_eq!(black.blue(), 0);

    let white = Tailwind::White.to_hdr_color().unwrap();
    assert_eq!(white.red(), 255);
    assert_eq!(white.green(), 255);
    assert_eq!(white.blue(), 255);
}

/// Test that black/white hex values are correct
#[test]
fn tailwind_black_white_hex() {
    assert_eq!(Tailwind::Black.hex(), Some("#000000"));
    assert_eq!(Tailwind::White.hex(), Some("#ffffff"));
}

/// Test sample reference colors against Tailwind v4 official values.
/// Official hex values from tailwindcss.com/docs/colors:
/// - slate-50: #f8fafc
/// - slate-500: #64748b
/// - slate-950: #020617
/// - red-500: #ef4444
/// - blue-500: #3b82f6
/// - indigo-500: #6366f1
#[test]
fn tailwind_reference_color_accuracy() {
    // Slate family
    let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
    assert_eq!(slate_50.red(), 248);
    assert_eq!(slate_50.green(), 250);
    assert_eq!(slate_50.blue(), 252);
    assert_eq!(Tailwind::Slate50.hex(), Some("#f8fafc"));

    let slate_500 = Tailwind::Slate500.to_hdr_color().unwrap();
    // Tailwind v4 uses OKLCH, so exact hex may differ slightly
    // The official slate-500 from Tailwind v4 is oklch(0.554 0.046 257.417)
    // Our conversion should be very close to #62748e (the sRGB fallback)
    assert!(
        (slate_500.red() as i16 - 100).abs() < 5,
        "slate-500 red should be ~100"
    );
    assert!(
        (slate_500.green() as i16 - 116).abs() < 5,
        "slate-500 green should be ~116"
    );
    assert!(
        (slate_500.blue() as i16 - 139).abs() < 5,
        "slate-500 blue should be ~139"
    );

    let slate_950 = Tailwind::Slate950.to_hdr_color().unwrap();
    assert!(
        (slate_950.red() as i16).abs() < 10,
        "slate-950 red should be ~2"
    );
    assert!(
        (slate_950.green() as i16 - 6).abs() < 10,
        "slate-950 green should be ~6"
    );
    assert!(
        (slate_950.blue() as i16 - 23).abs() < 10,
        "slate-950 blue should be ~23"
    );

    // Red-500: oklch(0.637 0.237 25.331) -> approximately #ef4444
    let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
    assert!(red_500.red() > 220, "red-500 should have high red channel");
    assert!(
        red_500.green() < 100,
        "red-500 should have low green channel"
    );
    assert!(red_500.blue() < 100, "red-500 should have low blue channel");

    // Blue-500: oklch(0.623 0.214 259.815) -> approximately #3b82f6
    let blue_500 = Tailwind::Blue500.to_hdr_color().unwrap();
    assert!(blue_500.red() < 100, "blue-500 should have low red channel");
    assert!(
        blue_500.green() > 100 && blue_500.green() < 150,
        "blue-500 green should be ~130"
    );
    assert!(
        blue_500.blue() > 230,
        "blue-500 should have high blue channel"
    );

    // Indigo-500: oklch(0.585 0.233 277.117) -> approximately #6366f1
    let indigo_500 = Tailwind::Indigo500.to_hdr_color().unwrap();
    assert!(
        indigo_500.red() > 80 && indigo_500.red() < 120,
        "indigo-500 red should be ~99"
    );
    assert!(
        indigo_500.green() > 80 && indigo_500.green() < 130,
        "indigo-500 green should be ~102"
    );
    assert!(
        indigo_500.blue() > 200,
        "indigo-500 should have high blue channel"
    );
}

/// Test that all concrete color variants return Some for to_hdr_color
#[test]
fn all_concrete_colors_return_some() {
    // Test a sample from each family
    let families = [
        Tailwind::Slate500,
        Tailwind::Gray500,
        Tailwind::Zinc500,
        Tailwind::Neutral500,
        Tailwind::Stone500,
        Tailwind::Red500,
        Tailwind::Orange500,
        Tailwind::Amber500,
        Tailwind::Yellow500,
        Tailwind::Lime500,
        Tailwind::Green500,
        Tailwind::Emerald500,
        Tailwind::Teal500,
        Tailwind::Cyan500,
        Tailwind::Sky500,
        Tailwind::Blue500,
        Tailwind::Indigo500,
        Tailwind::Violet500,
        Tailwind::Purple500,
        Tailwind::Fuchsia500,
        Tailwind::Pink500,
        Tailwind::Rose500,
    ];

    for color in families {
        assert!(
            color.to_hdr_color().is_some(),
            "{:?} should return Some for to_hdr_color",
            color
        );
        assert!(
            color.hex().is_some(),
            "{:?} should return Some for hex",
            color
        );
        assert!(
            !color.css_var().is_empty(),
            "{:?} should have non-empty css_var",
            color
        );
    }
}

/// Test OKLCH values are stored correctly
#[test]
fn oklch_values_preserved() {
    let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
    let (l, c, h) = slate_50.oklch();
    assert!((l - 0.984).abs() < 0.001, "Lightness should be ~0.984");
    assert!((c - 0.003).abs() < 0.001, "Chroma should be ~0.003");
    assert!((h - 247.858).abs() < 0.1, "Hue should be ~247.858");

    let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
    let (l, c, h) = red_500.oklch();
    assert!((l - 0.637).abs() < 0.001, "Red-500 L should be ~0.637");
    assert!((c - 0.237).abs() < 0.001, "Red-500 C should be ~0.237");
    assert!((h - 25.331).abs() < 0.1, "Red-500 H should be ~25.331");
}

/// Test CSS variable names follow Tailwind convention
#[test]
fn css_var_names_follow_convention() {
    assert_eq!(Tailwind::Black.css_var(), "--color-black");
    assert_eq!(Tailwind::White.css_var(), "--color-white");
    assert_eq!(Tailwind::Slate50.css_var(), "--color-slate-50");
    assert_eq!(Tailwind::Slate500.css_var(), "--color-slate-500");
    assert_eq!(Tailwind::Red500.css_var(), "--color-red-500");
    assert_eq!(Tailwind::Blue500.css_var(), "--color-blue-500");
}

/// Test hex values are properly formatted
#[test]
fn hex_format_is_valid() {
    let hex = Tailwind::Slate500.hex().unwrap();
    assert!(hex.starts_with('#'), "Hex should start with #");
    assert_eq!(hex.len(), 7, "Hex should be 7 characters (#rrggbb)");
    assert!(
        hex[1..].chars().all(|c| c.is_ascii_hexdigit()),
        "Hex should contain only hex digits"
    );
}

/// Test that neutral grays are truly achromatic (no color cast)
#[test]
fn neutral_grays_are_achromatic() {
    for shade in [
        Tailwind::Neutral50,
        Tailwind::Neutral100,
        Tailwind::Neutral200,
        Tailwind::Neutral300,
        Tailwind::Neutral400,
        Tailwind::Neutral500,
        Tailwind::Neutral600,
        Tailwind::Neutral700,
        Tailwind::Neutral800,
        Tailwind::Neutral900,
        Tailwind::Neutral950,
    ] {
        let color = shade.to_hdr_color().unwrap();
        let r = color.red() as i16;
        let g = color.green() as i16;
        let b = color.blue() as i16;

        // For truly neutral colors, R, G, and B should be identical
        // Allow small rounding tolerance
        assert!(
            (r - g).abs() <= 1 && (g - b).abs() <= 1,
            "{:?} should be achromatic: RGB({}, {}, {})",
            shade,
            r,
            g,
            b
        );
    }
}

/// Test fallback colors are appropriate for the shade
#[test]
fn fallback_colors_appropriate() {
    // Light colors (50, 100) should fall back to bright white
    let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
    assert_eq!(slate_50.fallback(), BasicColor::BrightWhite);

    // Mid-light colors (200, 300) should fall back to white
    let slate_200 = Tailwind::Slate200.to_hdr_color().unwrap();
    assert_eq!(slate_200.fallback(), BasicColor::White);

    // Mid colors (400-700) should fall back to bright black
    let slate_500 = Tailwind::Slate500.to_hdr_color().unwrap();
    assert_eq!(slate_500.fallback(), BasicColor::BrightBlack);

    // Dark colors (800-950) should fall back to black
    let slate_950 = Tailwind::Slate950.to_hdr_color().unwrap();
    assert_eq!(slate_950.fallback(), BasicColor::Black);

    // Colorful 500s should fall back to their basic color
    let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
    assert_eq!(red_500.fallback(), BasicColor::Red);

    let blue_500 = Tailwind::Blue500.to_hdr_color().unwrap();
    assert_eq!(blue_500.fallback(), BasicColor::Blue);

    let green_500 = Tailwind::Green500.to_hdr_color().unwrap();
    assert_eq!(green_500.fallback(), BasicColor::Green);
}

// ==========================================================================
// TailwindColorWrapper tests
// ==========================================================================

/// Test that TailwindColorWrapper::render produces correct ANSI escape sequence
#[test]
fn tailwind_wrapper_render_produces_correct_ansi() {
    let wrapper = TailwindColorWrapper(Tailwind::Indigo500);
    let output = wrapper.render("test");

    // Should start with ESC[38;2; for 24-bit color
    assert!(output.starts_with("\x1b[38;2;"));
    // Should end with ESC[0m (reset)
    assert!(output.ends_with("\x1b[0m"));
    // Should contain the content
    assert!(output.contains("test"));
}

/// Test that render output format matches expected pattern for known color
#[test]
fn tailwind_wrapper_render_format_for_known_color() {
    let wrapper = TailwindColorWrapper(Tailwind::Black);
    let output = wrapper.render("hello");

    // Black is RGB(0,0,0)
    assert_eq!(output, "\x1b[38;2;0;0;0mhello\x1b[0m");

    let wrapper = TailwindColorWrapper(Tailwind::White);
    let output = wrapper.render("world");

    // White is RGB(255,255,255)
    assert_eq!(output, "\x1b[38;2;255;255;255mworld\x1b[0m");
}

/// Test that special values return content unchanged
#[test]
fn tailwind_wrapper_special_values_return_unchanged() {
    let inherit_wrapper = TailwindColorWrapper(Tailwind::Inherit);
    assert_eq!(inherit_wrapper.render("content"), "content");

    let current_wrapper = TailwindColorWrapper(Tailwind::Current);
    assert_eq!(current_wrapper.render("content"), "content");

    let transparent_wrapper = TailwindColorWrapper(Tailwind::Transparent);
    assert_eq!(transparent_wrapper.render("content"), "content");
}

/// Test that special values don't panic in fallback_render
#[test]
fn tailwind_wrapper_special_values_fallback_no_panic() {
    let term = Terminal::new();

    let inherit_wrapper = TailwindColorWrapper(Tailwind::Inherit);
    let result = inherit_wrapper.fallback_render("test", &term);
    assert_eq!(result, "test");

    let current_wrapper = TailwindColorWrapper(Tailwind::Current);
    let result = current_wrapper.fallback_render("test", &term);
    assert_eq!(result, "test");

    let transparent_wrapper = TailwindColorWrapper(Tailwind::Transparent);
    let result = transparent_wrapper.fallback_render("test", &term);
    assert_eq!(result, "test");
}

/// Test fallback rendering respects TrueColor depth
#[test]
fn tailwind_wrapper_fallback_truecolor() {
    use crate::discovery::detection::ColorDepth;

    let mut term = Terminal::new();
    term.color_depth = ColorDepth::TrueColor;

    let wrapper = TailwindColorWrapper(Tailwind::Red500);
    let output = wrapper.fallback_render("error", &term);

    // Should use 24-bit truecolor format
    assert!(output.starts_with("\x1b[38;2;"));
    assert!(output.ends_with("\x1b[0m"));
    assert!(output.contains("error"));
}

/// Test fallback rendering respects Enhanced (256-color) depth
#[test]
fn tailwind_wrapper_fallback_enhanced() {
    use crate::discovery::detection::ColorDepth;

    let mut term = Terminal::new();
    term.color_depth = ColorDepth::Enhanced;

    let wrapper = TailwindColorWrapper(Tailwind::Blue500);
    let output = wrapper.fallback_render("info", &term);

    // Should use 256-color format: ESC[38;5;<n>m
    assert!(output.starts_with("\x1b[38;5;"));
    assert!(output.ends_with("\x1b[0m"));
    assert!(output.contains("info"));
}

/// Test fallback rendering uses basic ANSI for minimal color depth
#[test]
fn tailwind_wrapper_fallback_basic() {
    use crate::discovery::detection::ColorDepth;

    let mut term = Terminal::new();
    term.color_depth = ColorDepth::Basic;

    let wrapper = TailwindColorWrapper(Tailwind::Red500);
    let output = wrapper.fallback_render("warning", &term);

    // Should use basic ANSI format: ESC[<n>m where n is 30-37 or 90-97
    // Red500 fallback is BasicColor::Red which is code 31
    assert_eq!(output, "\x1b[31mwarning\x1b[0m");
}

/// Test fallback rendering uses basic ANSI for no color support
#[test]
fn tailwind_wrapper_fallback_none() {
    use crate::discovery::detection::ColorDepth;

    let mut term = Terminal::new();
    term.color_depth = ColorDepth::None;

    let wrapper = TailwindColorWrapper(Tailwind::Green500);
    let output = wrapper.fallback_render("success", &term);

    // Should still output basic ANSI (the terminal just won't render it)
    // Green500 fallback is BasicColor::Green which is code 32
    assert_eq!(output, "\x1b[32msuccess\x1b[0m");
}

/// Test wrapper can render various Tailwind colors without panic
#[test]
fn tailwind_wrapper_renders_all_families() {
    let colors = [
        Tailwind::Slate500,
        Tailwind::Gray500,
        Tailwind::Zinc500,
        Tailwind::Neutral500,
        Tailwind::Stone500,
        Tailwind::Red500,
        Tailwind::Orange500,
        Tailwind::Amber500,
        Tailwind::Yellow500,
        Tailwind::Lime500,
        Tailwind::Green500,
        Tailwind::Emerald500,
        Tailwind::Teal500,
        Tailwind::Cyan500,
        Tailwind::Sky500,
        Tailwind::Blue500,
        Tailwind::Indigo500,
        Tailwind::Violet500,
        Tailwind::Purple500,
        Tailwind::Fuchsia500,
        Tailwind::Pink500,
        Tailwind::Rose500,
    ];

    for color in colors {
        let wrapper = TailwindColorWrapper(color);
        let output = wrapper.render("test");

        assert!(
            output.starts_with("\x1b[38;2;"),
            "{:?} should produce truecolor escape",
            color
        );
        assert!(
            output.ends_with("\x1b[0m"),
            "{:?} should end with reset",
            color
        );
    }
}

/// Test wrapper with empty content
#[test]
fn tailwind_wrapper_empty_content() {
    let wrapper = TailwindColorWrapper(Tailwind::Indigo500);
    let output = wrapper.render("");

    // Should still have escape codes even with empty content
    assert!(output.starts_with("\x1b[38;2;"));
    assert!(output.ends_with("\x1b[0m"));
}

/// Test wrapper with content containing escape sequences
#[test]
fn tailwind_wrapper_content_with_escapes() {
    let wrapper = TailwindColorWrapper(Tailwind::Red500);
    let input = "already \x1b[1mbold\x1b[0m text";
    let output = wrapper.render(input);

    // Should wrap the content with color escapes
    assert!(output.starts_with("\x1b[38;2;"));
    assert!(output.ends_with("\x1b[0m"));
    assert!(output.contains("already \x1b[1mbold\x1b[0m text"));
}
