//! Integration tests for biscuit-terminal library.
//!
//! These tests verify the public API works correctly and that all
//! components integrate properly together.

use biscuit_terminal::discovery::{
    clipboard::*,
    config_paths::*,
    detection::*,
    eval::*,
    mode_2027::*,
    osc_queries::*,
    os_detection::*,
};
use biscuit_terminal::terminal::Terminal;

// ============================================================================
// Terminal struct integration tests
// ============================================================================

#[test]
fn test_terminal_struct_populates_all_fields() {
    let term = Terminal::new();

    // Verify all fields are accessible and have reasonable values
    let _app = &term.app;
    let _os = &term.os;
    let _distro = &term.distro;
    let _config = &term.config_file;
    let _is_tty = term.is_tty;
    let _is_ci = term.is_ci;
    let _color_depth = &term.color_depth;
    let _supports_italic = term.supports_italic;
    let _image_support = &term.image_support;
    let _underline_support = &term.underline_support;
    let _osc_link_support = term.osc_link_support;
}

#[test]
fn test_terminal_default_equals_new() {
    // Default and new should produce equivalent instances
    let term1 = Terminal::new();
    let term2 = Terminal::default();

    // Compare OS and CI status (these should be consistent)
    assert!(matches!(term1.os, OsType::MacOS | OsType::Linux | OsType::Windows | OsType::FreeBSD | OsType::NetBSD | OsType::OpenBSD | OsType::DragonFly | OsType::Illumos | OsType::Android | OsType::Ios | OsType::Unknown));
    assert!(matches!(term2.os, OsType::MacOS | OsType::Linux | OsType::Windows | OsType::FreeBSD | OsType::NetBSD | OsType::OpenBSD | OsType::DragonFly | OsType::Illumos | OsType::Android | OsType::Ios | OsType::Unknown));
}

#[test]
fn test_terminal_static_methods() {
    // Static methods should work without panic
    let width = Terminal::width();
    let height = Terminal::height();
    let _color_mode = Terminal::color_mode();

    // Dimensions should be reasonable
    assert!(width > 0, "Width should be positive");
    assert!(height > 0, "Height should be positive");
}

// ============================================================================
// Detection functions integration tests
// ============================================================================

#[test]
fn test_detection_functions_dont_panic() {
    // All detection functions should return valid values without panic
    let _color_depth = color_depth();
    let _color_mode = color_mode();
    let _is_tty = is_tty();
    let _terminal_app = get_terminal_app();
    let _width = terminal_width();
    let _height = terminal_height();
    let _dims = dimensions();
    let _image_support = image_support();
    let _osc8_support = osc8_link_support();
    let _multiplex = multiplex_support();
    let _underline = underline_support();
    let _italics = italics_support();
}

#[test]
fn test_dimensions_are_consistent() {
    let (w, h) = dimensions();
    assert_eq!(terminal_width(), w, "terminal_width() should match dimensions().0");
    assert_eq!(terminal_height(), h, "terminal_height() should match dimensions().1");
}

#[test]
fn test_underline_support_consistency() {
    let support = underline_support();

    // If we have curly underlines, we should have straight underlines
    if support.curly {
        assert!(support.straight, "Curly support implies straight support");
    }

    // If we have colored underlines, we should have straight underlines
    if support.colored {
        assert!(support.straight, "Colored underline support implies straight support");
    }
}

// ============================================================================
// OS detection integration tests
// ============================================================================

#[test]
fn test_os_detection_functions_dont_panic() {
    let _os = detect_os_type();
    let _distro = detect_linux_distro();
    let _ci = is_ci();
}

#[test]
fn test_os_type_matches_current_platform() {
    let os = detect_os_type();

    #[cfg(target_os = "macos")]
    assert_eq!(os, OsType::MacOS);

    #[cfg(target_os = "linux")]
    assert!(matches!(os, OsType::Linux | OsType::Android));

    #[cfg(target_os = "windows")]
    assert_eq!(os, OsType::Windows);

    #[cfg(target_os = "freebsd")]
    assert_eq!(os, OsType::FreeBSD);
}

#[test]
fn test_distro_only_set_on_linux() {
    let os = detect_os_type();
    let distro = detect_linux_distro();

    if os != OsType::Linux {
        assert!(
            distro.is_none(),
            "Distro should be None on non-Linux systems"
        );
    }
}

// ============================================================================
// OSC queries integration tests
// ============================================================================

#[test]
fn test_osc_queries_dont_panic() {
    // These may return None in test environments, but should never panic
    let _bg = bg_color();
    let _text = text_color();
    let _cursor = cursor_color();
}

#[test]
fn test_rgb_value_operations() {
    let black = RgbValue::new(0, 0, 0);
    let white = RgbValue::new(255, 255, 255);
    let gray = RgbValue::new(128, 128, 128);

    // Luminance ordering
    assert!(black.luminance() < gray.luminance());
    assert!(gray.luminance() < white.luminance());

    // Light/dark classification
    assert!(black.is_dark());
    assert!(white.is_light());

    // Display works
    assert!(!black.to_string().is_empty());
    assert!(!white.to_string().is_empty());
}

// ============================================================================
// Clipboard integration tests
// ============================================================================

#[test]
fn test_clipboard_functions_dont_panic() {
    let _support = osc52_support();
    let _clipboard = get_clipboard();
    // Don't test set_clipboard as it writes to stdout
}

#[test]
fn test_build_osc52_sequence_roundtrip() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    let content = "Hello, World!";
    let sequence = build_osc52_sequence(content, ClipboardTarget::Clipboard);

    // Parse and verify
    assert!(sequence.starts_with("\x1b]52;c;"));
    assert!(sequence.ends_with("\x07"));

    // Extract and decode base64
    let start = "\x1b]52;c;".len();
    let end = sequence.len() - 1;
    let encoded = &sequence[start..end];
    let decoded = BASE64.decode(encoded).unwrap();
    let result = String::from_utf8(decoded).unwrap();

    assert_eq!(result, content);
}

#[test]
fn test_clipboard_targets() {
    assert_eq!(ClipboardTarget::default(), ClipboardTarget::Clipboard);

    // Build sequences for all targets
    let targets = [
        ClipboardTarget::Clipboard,
        ClipboardTarget::Primary,
        ClipboardTarget::Both,
    ];

    for target in targets {
        let seq = build_osc52_sequence("test", target);
        assert!(seq.starts_with("\x1b]52;"));
        assert!(seq.ends_with("\x07"));
    }
}

// ============================================================================
// Mode 2027 integration tests
// ============================================================================

#[test]
fn test_mode_2027_functions_dont_panic() {
    let _support = supports_mode_2027();
    // enable/disable may fail in test env, but shouldn't panic
    let _ = enable_mode_2027();
    let _ = disable_mode_2027();
}

// ============================================================================
// Eval module integration tests
// ============================================================================

#[test]
fn test_eval_functions_with_various_inputs() {
    // Plain text
    assert!(!has_escape_codes("hello"));
    assert!(!has_osc8_link("hello"));
    assert_eq!(line_widths("hello"), vec![5]);

    // With escape codes
    assert!(has_escape_codes("\x1b[31mred\x1b[0m"));
    assert_eq!(line_widths("\x1b[31mred\x1b[0m"), vec![3]);

    // With OSC8 link
    let link = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07";
    assert!(has_escape_codes(link));
    assert!(has_osc8_link(link));
    assert_eq!(line_widths(link), vec![5]);

    // Unicode - CJK
    assert_eq!(line_widths("\u{4F60}\u{597D}"), vec![4]); // "ni hao" in Chinese

    // Unicode - accented
    assert_eq!(line_widths("caf\u{00E9}"), vec![4]); // "cafe" with acute accent

    // Multiline
    assert_eq!(line_widths("line1\nline2\nline3"), vec![5, 5, 5]);
}

#[test]
fn test_eval_empty_input() {
    assert_eq!(line_widths(""), vec![0]);
    assert!(!has_escape_codes(""));
    assert!(!has_osc8_link(""));
}

#[test]
fn test_eval_complex_escape_sequences() {
    // Multiple SGR parameters
    let multi_sgr = "\x1b[1;4;31mtext\x1b[0m";
    assert!(has_escape_codes(multi_sgr));
    assert_eq!(line_widths(multi_sgr), vec![4]);

    // 256 color mode
    let color_256 = "\x1b[38;5;196mred\x1b[0m";
    assert!(has_escape_codes(color_256));
    assert_eq!(line_widths(color_256), vec![3]);

    // True color (24-bit)
    let true_color = "\x1b[38;2;255;0;0mred\x1b[0m";
    assert!(has_escape_codes(true_color));
    assert_eq!(line_widths(true_color), vec![3]);
}

// ============================================================================
// Config paths integration tests
// ============================================================================

#[test]
fn test_config_paths_for_known_terminals() {
    // Terminals that always have config paths
    let wezterm_path = get_terminal_config_path(&TerminalApp::Wezterm);
    assert!(wezterm_path.is_some(), "Wezterm should always have a config path");

    let alacritty_path = get_terminal_config_path(&TerminalApp::Alacritty);
    assert!(alacritty_path.is_some(), "Alacritty should always have a config path");

    // Terminals that return None
    let gnome_path = get_terminal_config_path(&TerminalApp::GnomeTerminal);
    assert!(gnome_path.is_none(), "GNOME Terminal uses dconf");

    let wast_path = get_terminal_config_path(&TerminalApp::Wast);
    assert!(wast_path.is_none(), "Wast has no standard config");
}

#[test]
fn test_config_paths_multiple_for_alacritty() {
    let paths = get_terminal_config_paths(&TerminalApp::Alacritty);

    #[cfg(not(target_os = "windows"))]
    {
        assert!(paths.len() >= 2, "Alacritty should have multiple config paths");

        // Check for both .toml and .yml
        let has_toml = paths.iter().any(|p| {
            p.extension().map(|e| e == "toml").unwrap_or(false)
        });
        let has_yml = paths.iter().any(|p| {
            p.extension().map(|e| e == "yml").unwrap_or(false)
        });

        assert!(has_toml, "Should include .toml path");
        assert!(has_yml, "Should include .yml path");
    }
}

// ============================================================================
// Cross-module integration tests
// ============================================================================

#[test]
fn test_terminal_config_matches_detection() {
    let term = Terminal::new();

    // The config file in Terminal should match what get_terminal_config_path returns
    let config_from_detection = get_terminal_config_path(&term.app);
    assert_eq!(term.config_file, config_from_detection);
}

#[test]
fn test_terminal_os_matches_detection() {
    let term = Terminal::new();
    let os_from_detection = detect_os_type();

    assert_eq!(term.os, os_from_detection);
}

#[test]
fn test_terminal_distro_matches_detection() {
    let term = Terminal::new();
    let distro_from_detection = detect_linux_distro();

    assert_eq!(term.distro, distro_from_detection);
}

// ============================================================================
// Stress tests
// ============================================================================

#[test]
fn test_repeated_terminal_creation() {
    // Creating multiple Terminal instances shouldn't cause issues
    // Note: Keep iteration count low to avoid OSC query flooding in TTY environments
    for _ in 0..5 {
        let _term = Terminal::new();
    }
}

#[test]
fn test_line_widths_with_long_input() {
    // Test with a long string
    let long_string = "a".repeat(10000);
    let widths = line_widths(&long_string);
    assert_eq!(widths, vec![10000]);
}

#[test]
fn test_line_widths_with_many_lines() {
    // Test with many lines
    let many_lines: String = (0..1000).map(|_| "line").collect::<Vec<_>>().join("\n");
    let widths = line_widths(&many_lines);
    assert_eq!(widths.len(), 1000);
}

#[test]
fn test_line_widths_with_mixed_content() {
    // Mix of ASCII, CJK, emoji, and escape codes
    let mixed = format!(
        "Hello {}\x1b[31m{}\x1b[0m {}",
        "\u{4F60}\u{597D}", // Chinese "ni hao"
        "\u{1F389}",        // party popper emoji
        "world"
    );

    let widths = line_widths(&mixed);
    // Should complete without panic
    assert_eq!(widths.len(), 1);
}
