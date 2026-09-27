//! Level 1 checks of the styled-capture parser the Level 2 tests assert
//! through: tmux's re-encoded SGR forms and its 256-color downgrade.

mod styled_capture;

use styled_capture::{Color, Style, StyledScreen};

#[test]
fn parses_merged_resets_and_both_truecolor_forms() {
    let screen = StyledScreen::parse(
        "\x1b[2;3mab\x1b[0m c\x1b[38;2;255;165;0md\x1b[39m\n\x1b[48:2::1:2:3m e\x1b[49m\x1b]8;;https://x\x1b\\f",
    );
    assert_eq!(screen.text(0), "ab cd");
    assert_eq!(screen.text(1), " ef");
    let a = screen.rows[0][0].style;
    assert!(a.dim && a.italic && a.fg.is_none());
    assert_eq!(screen.rows[0][2].style, Style::default());
    assert!(screen.rows[0][4].style.fg_is(Color::Rgb(255, 165, 0)));
    assert!(screen.rows[1][0].style.bg_is(Color::Rgb(1, 2, 3)));
    assert!(screen.rows[1][2].style.bg.is_none());
}

#[test]
fn a_256_color_downgrade_matches_its_rgb_source() {
    // tmux's own mapping: orange (255,165,0) is cube index 214.
    assert!(Color::Indexed(214).matches(Color::Rgb(255, 165, 0)));
    assert!(!Color::Indexed(226).matches(Color::Rgb(255, 165, 0)));
    assert!(!Color::Indexed(3).matches(Color::Rgb(255, 165, 0)));
}
