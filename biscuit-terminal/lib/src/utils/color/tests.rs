use super::*;
use crate::terminal::Terminal;
use crate::utils::layout::RenderableWrapper;

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
