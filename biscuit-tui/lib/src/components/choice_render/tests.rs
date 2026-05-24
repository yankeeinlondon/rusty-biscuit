use super::super::choose::HotkeySpec;
use super::badge::{
    ALT_BADGE_BG, ALT_BADGE_BG_DIM, BADGE_FG_ON_ORANGE, BADGE_FG_ON_YELLOW, CTRL_BADGE_BG,
    CTRL_BADGE_BG_DIM,
};
use super::*;
use crate::core::{TerminalBackground, resolve_active_style};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

fn default_theme() -> ComponentTheme {
    ComponentTheme::default()
}

fn buffer_row(buf: &Buffer, y: u16) -> String {
    let mut row = String::new();
    for x in buf.area.left()..buf.area.right() {
        row.push_str(buf[(x, y)].symbol());
    }
    row.trim_end().to_string()
}

#[test]
fn render_vertical_draws_indicator_and_label() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical);
    let options = vec![
        ChoiceOption::<String>::new("r", "Red", "red"),
        ChoiceOption::<String>::new("g", "Green", "green"),
    ];
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    assert!(buffer_row(&buf, 0).starts_with("▶ ○ Red"));
    assert!(buffer_row(&buf, 1).starts_with("  ○ Green"));
}

#[test]
fn render_vertical_selected_indicator() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical);
    let options = vec![
        ChoiceOption::<String>::new("r", "Red", "red"),
        ChoiceOption::<String>::new("g", "Green", "green"),
    ];
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1],
        0,
        1,
        |idx| idx == 0,
        None,
        None,
    );

    assert!(buffer_row(&buf, 0).starts_with("  ● Red"));
    assert!(buffer_row(&buf, 1).starts_with("▶ ○ Green"));
}

#[test]
fn render_multiple_checkbox_indicators() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_multiple(&theme, TerminalStyle::default(), Orientation::Vertical);
    let options = vec![
        ChoiceOption::<String>::new("a", "Alpha", "alpha"),
        ChoiceOption::<String>::new("b", "Beta", "beta"),
    ];
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1],
        0,
        0,
        |idx| idx == 0,
        None,
        None,
    );

    assert!(buffer_row(&buf, 0).starts_with("▶ ☑ Alpha"));
    assert!(buffer_row(&buf, 1).starts_with("  ☐ Beta"));
}

#[test]
fn render_hover_background_covers_prefix_and_label_plus_one() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical);
    let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    let hover_style =
        resolve_active_style(ActiveChoiceColor::default(), TerminalBackground::default());
    // Prefix "▶ ○ " = 4 cells, label "Red" = 3 cells, trailing " " = 1 cell = 8 cells total.
    for x in 0..8 {
        let style = buf[(x, 0)].style();
        assert_eq!(style.fg, hover_style.fg, "cell {x} should have correct fg");
        assert_eq!(style.bg, hover_style.bg, "cell {x} should have correct bg");
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "cell {x} should have BOLD modifier"
        );
    }
    // Everything beyond the content should keep default style.
    let style = buf[(9, 0)].style();
    assert!(
        style.fg != hover_style.fg || style.bg != hover_style.bg,
        "cell 9 should not have hover background"
    );
}

/// Renders a single-row choice list at the given background and
/// returns the resulting buffer for assertion.
fn render_single_active_row(
    background: TerminalBackground,
    active_color: ActiveChoiceColor,
) -> Buffer {
    let theme = default_theme();
    let term = TerminalStyle {
        background,
        ..TerminalStyle::default()
    };
    let ctx = ChoiceRenderContext::for_single(&theme, term, Orientation::Vertical)
        .with_active_color(active_color);
    let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| false,
        None,
        None,
    );
    buf
}

#[test]
fn active_row_uses_grey_default_palette_on_dark_background() {
    let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
    // Index 1 lands on the focus indicator's trailing space — part
    // of the styled prefix that should carry the active background.
    assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(238)));
}

#[test]
fn active_row_uses_green_palette_when_configured() {
    let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Green);
    assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(22)));
}

#[test]
fn active_row_uses_white_fg_on_dark_bg() {
    let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
    assert_eq!(buf[(1, 0)].style().fg, Some(Color::White));
}

#[test]
fn active_row_uses_black_fg_on_light_bg() {
    let buf = render_single_active_row(TerminalBackground::Light, ActiveChoiceColor::Grey);
    assert_eq!(buf[(1, 0)].style().fg, Some(Color::Black));
    assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(252)));
}

#[test]
fn active_row_uses_dark_palette_on_unknown_background() {
    let unknown = render_single_active_row(TerminalBackground::Unknown, ActiveChoiceColor::Red);
    let dark = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Red);
    assert_eq!(unknown[(1, 0)].style().bg, dark[(1, 0)].style().bg);
    assert_eq!(unknown[(1, 0)].style().fg, dark[(1, 0)].style().fg);
    assert_eq!(unknown[(1, 0)].style().bg, Some(Color::Indexed(52)));
}

#[test]
fn active_row_style_covers_only_label_plus_one_blank() {
    // Prefix "▶ ○ " = 4 cells, label "Red" = 3 cells, trailing
    // blank = 1 cell. So cells 0..8 must carry the active style and
    // cell 8 onwards must be unstyled. (Cell index 8 is the first
    // unstyled cell; cell 9 is "well past" the styled span.)
    let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
    let active_bg = Some(Color::Indexed(238));
    for x in 0..8 {
        assert_eq!(
            buf[(x, 0)].style().bg,
            active_bg,
            "cell {x} should carry the active background"
        );
    }
    for x in 8..20 {
        assert_ne!(
            buf[(x, 0)].style().bg,
            active_bg,
            "cell {x} must NOT carry the active background"
        );
    }
}

#[test]
fn active_row_style_does_not_underline() {
    let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
    let modifier = buf[(1, 0)].style().add_modifier;
    assert!(modifier.contains(Modifier::BOLD));
    assert!(!modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn render_disabled_row_uses_disabled_style() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical);
    let mut options = vec![
        ChoiceOption::<String>::new("a", "Active", "active"),
        ChoiceOption::<String>::new("d", "Disabled", "disabled"),
    ];
    options[1].disabled = true;
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1],
        0,
        1,
        |_idx| false,
        None,
        None,
    );

    // Find the 'D' in "Disabled" and verify it has disabled style.
    let mut found = false;
    for x in 0..area.width {
        let cell = &buf[(x, 1)];
        if cell.symbol() == "D" {
            assert!(
                cell.style().fg == Some(Color::DarkGray)
                    || cell.style().add_modifier.contains(Modifier::DIM),
                "Disabled label should have disabled style"
            );
            found = true;
            break;
        }
    }
    assert!(found, "Did not find 'D' in buffer");
}

#[test]
fn render_overflow_indicators_when_scrolled() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical);
    let options: Vec<ChoiceOption> = (0..5)
        .map(|i| {
            ChoiceOption::<String>::new(format!("id{i}"), format!("Option {i}"), format!("val{i}"))
        })
        .collect();
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2, 3, 4],
        1, // scrolled down by 1
        2,
        |_idx| false,
        None,
        None,
    );

    let top_row = buffer_row(&buf, 0);
    assert!(top_row.contains("▲"), "Expected up overflow indicator");

    let bottom_row = buffer_row(&buf, 1);
    assert!(bottom_row.contains("▼"), "Expected down overflow indicator");
}

#[test]
fn render_no_matches_when_filter_empty() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical);
    let options = vec![
        ChoiceOption::<String>::new("a", "Apple", "apple"),
        ChoiceOption::<String>::new("b", "Banana", "banana"),
    ];
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);

    let mut filter = FuzzyFilter::new();
    filter.clear(&["Apple".into(), "Banana".into()]);
    filter.push_char('z', &["Apple".into(), "Banana".into()]);
    let visible_indices: Vec<usize> = filter.visible().to_vec();

    ctx.render(
        area,
        &mut buf,
        &options,
        &visible_indices,
        0,
        0,
        |_idx| false,
        Some(&mut filter),
        None,
    );

    assert_eq!(buffer_row(&buf, 0), "(no matches)");
}

#[test]
fn build_highlighted_spans_empty_highlights() {
    let base = Style::default().fg(Color::White);
    let match_style = Style::default().fg(Color::Yellow);
    let spans = build_highlighted_spans("hello", &[], base, match_style);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "hello");
    assert_eq!(spans[0].style, base);
}

#[test]
fn build_highlighted_spans_multibyte_chars() {
    let base = Style::default().fg(Color::White);
    let match_style = Style::default().fg(Color::Yellow);
    let spans = build_highlighted_spans("Café", &[0, 1], base, match_style);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content, "Ca");
    assert_eq!(spans[0].style, match_style);
    assert_eq!(spans[1].content, "fé");
    assert_eq!(spans[1].style, base);
}

#[test]
fn render_single_uses_fallback_glyphs_when_no_nerd_font() {
    let theme = default_theme();
    let ctx = ChoiceRenderContext::for_single(
        &theme,
        TerminalStyle {
            nerd_font: NerdFontStatus::Unknown,
            ..TerminalStyle::default()
        },
        Orientation::Vertical,
    );
    let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| true,
        None,
        None,
    );

    let row = buffer_row(&buf, 0);
    assert!(
        row.contains("●"),
        "expected fallback filled radio ●, got: {row}"
    );
}

#[test]
fn render_single_uses_nerd_font_glyphs_when_likely() {
    let theme = default_theme();
    let ctx = ChoiceRenderContext::for_single(
        &theme,
        TerminalStyle {
            nerd_font: NerdFontStatus::Likely,
            ..TerminalStyle::default()
        },
        Orientation::Vertical,
    );
    let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| true,
        None,
        None,
    );

    let row = buffer_row(&buf, 0);
    assert!(
        row.contains('\u{f043e}'),
        "expected Nerd Font filled radio \\u{{f043e}}, got: {row}"
    );
}

#[test]
fn render_multiple_uses_fallback_glyphs_when_no_nerd_font() {
    let theme = default_theme();
    let ctx = ChoiceRenderContext::for_multiple(
        &theme,
        TerminalStyle {
            nerd_font: NerdFontStatus::Unknown,
            ..TerminalStyle::default()
        },
        Orientation::Vertical,
    );
    let options = vec![ChoiceOption::<String>::new("a", "Alpha", "alpha")];
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| true,
        None,
        None,
    );

    let row = buffer_row(&buf, 0);
    assert!(
        row.contains("☑"),
        "expected fallback checked box ☑, got: {row}"
    );
}

#[test]
fn render_multiple_uses_nerd_font_glyphs_when_likely() {
    let theme = default_theme();
    let ctx = ChoiceRenderContext::for_multiple(
        &theme,
        TerminalStyle {
            nerd_font: NerdFontStatus::Likely,
            ..TerminalStyle::default()
        },
        Orientation::Vertical,
    );
    let options = vec![ChoiceOption::<String>::new("a", "Alpha", "alpha")];
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| true,
        None,
        None,
    );

    let row = buffer_row(&buf, 0);
    assert!(
        row.contains('\u{f14a}'),
        "expected Nerd Font checked box \\u{{f14a}}, got: {row}"
    );
}

// --- Phase 6: hotkey badges ----------------------------------------

/// Returns the column at which the badge for option `idx` first
/// appears in `buf` row `y`, or `None` when no Ctrl/Alt badge
/// background colour is detected.
fn find_badge_x(buf: &Buffer, y: u16, expected_bg: Color) -> Option<u16> {
    (0..buf.area.width).find(|&x| buf[(x, y)].style().bg == Some(expected_bg))
}

fn fixture_with_hotkeys() -> Vec<ChoiceOption<String>> {
    vec![
        ChoiceOption::<String>::new("r", "Red", "red")
            .with_hotkey(super::super::choose::HotkeySpec::Ctrl('r')),
        ChoiceOption::<String>::new("g", "Green", "green")
            .with_hotkey(super::super::choose::HotkeySpec::Alt('g')),
        ChoiceOption::<String>::new("b", "Blue", "blue"),
    ]
}

#[test]
fn vertical_render_draws_ctrl_badge_with_orange_bg_and_bold_text() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options = fixture_with_hotkeys();
    let area = Rect::new(0, 0, 30, 3);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    // Row 0 (Red) carries a Ctrl badge in orange (208) with
    // BLACK foreground — high-contrast on bright orange (white
    // on orange has marginal contrast on many terminal palettes).
    let badge_x =
        find_badge_x(&buf, 0, CTRL_BADGE_BG).expect("expected Ctrl badge background on row 0");
    let style = buf[(badge_x, 0)].style();
    assert_eq!(style.fg, Some(BADGE_FG_ON_ORANGE));
    assert_eq!(style.bg, Some(CTRL_BADGE_BG));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn vertical_render_draws_alt_badge_with_yellow_bg_and_bold_text() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::AltHeld);
    let options = fixture_with_hotkeys();
    let area = Rect::new(0, 0, 30, 3);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    // Row 1 (Green) carries an Alt badge in yellow (220) with
    // BLACK foreground — white-on-yellow has poor contrast and
    // reads as a yellow blur on most terminal themes.
    let badge_x =
        find_badge_x(&buf, 1, ALT_BADGE_BG).expect("expected Alt badge background on row 1");
    let style = buf[(badge_x, 1)].style();
    assert_eq!(style.fg, Some(BADGE_FG_ON_YELLOW));
    assert_eq!(style.bg, Some(ALT_BADGE_BG));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn vertical_render_when_ctrl_held_renders_alt_badges_with_dim_bg() {
    // Under `CtrlHeld`, Alt badges still appear so the user can
    // see all bound hotkeys at a glance — **including a coloured
    // background fill** (per spec). The visual differentiator
    // between held and not-held is the darker BG shade plus the
    // absence of BOLD, NOT removal of the BG. We deliberately do
    // not use `Modifier::DIM` because it renders inconsistently
    // across terminals.
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options = fixture_with_hotkeys();
    let area = Rect::new(0, 0, 30, 3);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    // Row 1 (Green) has an Alt hotkey. Under CtrlHeld it renders
    // with the *dim* Alt BG, BLACK FG (yellow-family), and no BOLD.
    let badge_x = find_badge_x(&buf, 1, ALT_BADGE_BG_DIM)
        .expect("expected Alt badge in dim-BG treatment under CtrlHeld");
    let style = buf[(badge_x, 1)].style();
    assert_eq!(style.fg, Some(BADGE_FG_ON_YELLOW));
    assert_eq!(style.bg, Some(ALT_BADGE_BG_DIM));
    assert!(!style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn vertical_badges_align_horizontally_across_hovered_and_non_hovered_rows() {
    // Regression: the active (hovered) row used to render an extra
    // hover-styled trailing blank that pushed its badge one cell
    // further right than badges on non-hovered rows. All badges
    // must share the same starting x-column.
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_multiple(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options: Vec<ChoiceOption<String>> = vec![
        ChoiceOption::new("a", "foo", "foo").with_hotkey(HotkeySpec::Ctrl('1')),
        ChoiceOption::new("b", "bar", "bar").with_hotkey(HotkeySpec::Ctrl('2')),
        ChoiceOption::new("c", "baz", "baz").with_hotkey(HotkeySpec::Ctrl('3')),
        ChoiceOption::new("d", "bax", "bax").with_hotkey(HotkeySpec::Ctrl('4')),
    ];
    let area = Rect::new(0, 0, 30, 4);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2, 3],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    let xs: Vec<u16> = (0..4)
        .map(|y| {
            find_badge_x(&buf, y, CTRL_BADGE_BG)
                .or_else(|| find_badge_x(&buf, y, CTRL_BADGE_BG_DIM))
                .unwrap_or_else(|| panic!("expected a Ctrl badge on row {y}"))
        })
        .collect();
    assert!(
        xs.windows(2).all(|w| w[0] == w[1]),
        "badges must share the same x-column across rows; got {xs:?}"
    );
}

#[test]
fn vertical_render_hidden_mode_omits_badges() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::Hidden);
    let options = fixture_with_hotkeys();
    let area = Rect::new(0, 0, 30, 3);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    assert!(find_badge_x(&buf, 0, CTRL_BADGE_BG).is_none());
    assert!(find_badge_x(&buf, 1, ALT_BADGE_BG).is_none());
}

#[test]
fn no_badge_for_options_without_explicit_hotkey() {
    // Auto-derivation is gone — an option without an explicit
    // `with_hotkey()` call renders no badge, even when the user
    // is holding Ctrl.
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options = vec![ChoiceOption::<String>::new("b", "Blue", "blue")];
    let area = Rect::new(0, 0, 30, 1);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    assert!(find_badge_x(&buf, 0, CTRL_BADGE_BG).is_none());
    assert!(find_badge_x(&buf, 0, ALT_BADGE_BG).is_none());
}

#[test]
fn disabled_options_do_not_render_default_badges() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Vertical)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options = vec![ChoiceOption::<String>::new("b", "Blue", "blue").disabled()];
    let area = Rect::new(0, 0, 30, 1);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    assert!(find_badge_x(&buf, 0, CTRL_BADGE_BG).is_none());
}

#[test]
fn horizontal_layout_measures_explicit_badges() {
    // Badge width must NOT inflate the layout width in horizontal mode
    // because badges render on a sub-row below the option, not inline.
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Horizontal)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options = vec![
        ChoiceOption::<String>::new("a", "Alpha", "alpha").with_hotkey(HotkeySpec::Ctrl('a')),
        ChoiceOption::<String>::new("b", "Bravo", "bravo").with_hotkey(HotkeySpec::Ctrl('b')),
    ];
    let area = Rect::new(0, 0, 12, 4);
    let layout = ctx.compute_layout(area, &options, &[0, 1], |_idx| false);

    // Content-only width: indicator(1) + space(1) + "Alpha"(5) + trailing(1) = 8.
    // Badge width (^A = 2 cells) is excluded from layout.
    assert_eq!(layout.item_rects[0].width, 8);
    // 8 + gap(1) + 8 = 17 > 12, so items wrap to two rows.
    assert_eq!(layout.row_count(), 2);
    assert_eq!(layout.item_rects[1].y, 2);
}

#[test]
fn horizontal_render_places_badge_below_row_not_inline() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Horizontal)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options = fixture_with_hotkeys();
    // 30 cols wide × 4 rows tall: enough for a single horizontal
    // row plus one badge row beneath it.
    let area = Rect::new(0, 0, 60, 4);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    // The Ctrl badge for option 0 (Red) must appear on row 1
    // (immediately below the option row), not on row 0.
    assert!(
        find_badge_x(&buf, 0, CTRL_BADGE_BG).is_none(),
        "Ctrl badge must NOT appear inline on the option row in horizontal mode"
    );
    assert!(
        find_badge_x(&buf, 1, CTRL_BADGE_BG).is_some(),
        "Ctrl badge must appear on the row directly below the option in horizontal mode"
    );
}

#[test]
fn horizontal_multi_row_badges_do_not_overwrite_next_row_options() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Horizontal)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options: Vec<ChoiceOption<String>> = vec![
        ChoiceOption::new("a", "Alpha", "alpha").with_hotkey(HotkeySpec::Ctrl('a')),
        ChoiceOption::new("b", "Bravo", "bravo").with_hotkey(HotkeySpec::Ctrl('b')),
        ChoiceOption::new("c", "Charlie", "charlie").with_hotkey(HotkeySpec::Ctrl('c')),
        ChoiceOption::new("d", "Delta", "delta").with_hotkey(HotkeySpec::Ctrl('d')),
    ];
    // Narrow width forces wrapping: each option is ~11-13 cells
    // (indicator + space + label + trailing blank + badge padding + badge),
    // so at most one option per row. With row_height=2 (badges visible),
    // row 0 option occupies y=0, badge at y=1; row 1 option at y=2,
    // badge at y=3.
    let area = Rect::new(0, 0, 15, 8);
    let mut buf = Buffer::empty(area);
    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2, 3],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    // Row 0 (y=0): option text for "Alpha"
    // Row 1 (y=1): badge sub-row for row 0 — must NOT contain
    //              option label text from subsequent options.
    // Row 2 (y=2): option text for "Bravo"
    // Row 3 (y=3): badge sub-row for row 1

    // Verify that the badge row (y=1) does not contain option
    // label characters from subsequent rows.
    let badge_row = buffer_row(&buf, 1);
    assert!(
        !badge_row.contains("Bravo"),
        "badge row y=1 must not contain option text from row 1: got '{badge_row}'"
    );
    assert!(
        !badge_row.contains("Charlie"),
        "badge row y=1 must not contain option text from row 2: got '{badge_row}'"
    );

    // Verify that the option row (y=2) contains option text (Bravo).
    let option_row_1 = buffer_row(&buf, 2);
    assert!(
        option_row_1.contains("Bravo"),
        "option row y=2 must contain its option text: got '{option_row_1}'"
    );

    // Verify badge appears on y=3 (below the second option row),
    // not colliding with the option content on y=2.
    assert!(
        find_badge_x(&buf, 3, CTRL_BADGE_BG).is_some(),
        "badge for row 1 option must appear on y=3"
    );

    // Cross-check: option text on y=2 must NOT have badge background.
    assert!(
        find_badge_x(&buf, 2, CTRL_BADGE_BG).is_none(),
        "option row y=2 must not carry badge background (collision)"
    );

    // Verify badge row y=3 does not overwrite option text on y=4.
    let option_row_2 = buffer_row(&buf, 4);
    assert!(
        option_row_2.contains("Charlie"),
        "option row y=4 must contain Charlie: got '{option_row_2}'"
    );
    assert!(
        find_badge_x(&buf, 4, CTRL_BADGE_BG).is_none(),
        "option row y=4 must not carry badge background (collision)"
    );
}

#[test]
fn horizontal_badges_visible_count_uses_logical_rows() {
    let theme = default_theme();
    let ctx =
        ChoiceRenderContext::for_single(&theme, TerminalStyle::default(), Orientation::Horizontal)
            .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let options: Vec<ChoiceOption<String>> = vec![
        ChoiceOption::new("a", "Alpha", "alpha").with_hotkey(HotkeySpec::Ctrl('a')),
        ChoiceOption::new("b", "Bravo", "bravo").with_hotkey(HotkeySpec::Ctrl('b')),
        ChoiceOption::new("c", "Charlie", "charlie").with_hotkey(HotkeySpec::Ctrl('c')),
    ];
    let area = Rect::new(0, 0, 12, 3);
    let mut buf = Buffer::empty(area);

    ctx.render(
        area,
        &mut buf,
        &options,
        &[0, 1, 2],
        0,
        0,
        |_idx| false,
        None,
        None,
    );

    assert_eq!(ctx.row_height(), 2);
    assert_eq!(ctx.visible_logical_rows(area.height), 1);
    assert!(buffer_row(&buf, 0).contains("Alpha"));
    assert!(
        find_badge_x(&buf, 1, CTRL_BADGE_BG).is_some(),
        "first logical row badge should render on y=1"
    );
    for y in 0..area.height {
        let row = buffer_row(&buf, y);
        assert!(
            !row.contains("Bravo") && !row.contains("Charlie"),
            "short viewport must not render hidden logical rows on y={y}: {row:?}"
        );
    }
}
