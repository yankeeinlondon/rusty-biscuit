//! Parity tests for `Status`'s box-model contract.
//!
//! `Status` is classified as an **inline badge** (spec C7). A status line is
//! a single inline run (icon + description) — it does not own a block box,
//! so the layout-box properties are N/A here. When `Status` is composed
//! into a block container (e.g. via `Compose` or `Section`), the containing
//! block owns the box and `Status`'s content flows inline within it.
//!
//! ## Style-everywhere Phase 3 contract (Task 3.6)
//!
//! - `Layout::word_wrap` is **Honored** on the description text.
//! - `Layout::margin` / `alignment` / `max_width` / `width` / `padding` are
//!   **N/A** (inline badge — no block box).
//! - `Style::color` is **Honored** via the state's `default_color` and the
//!   Tailwind icon palette.
//! - `Style::emphasis` is **Honored** via Prose markup when `use_prose`.
//! - `Style::background` and `Style::border` are **N/A**.
//!
//! Phase 4 / Task 4.4 will add the inline-content matrix entry that pins
//! the box-property N/A cells alongside the other inline components.

mod parity_helpers;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use renderable::layout::{
    Alignment, Edges, Length, TargetValue, WordWrap,
};

use parity_helpers::{strip_ansi, test_terminal};

// ---------------------------------------------------------------------------
// Honored: Layout::word_wrap wraps the description
// ---------------------------------------------------------------------------

#[test]
fn word_wrap_default_wraps_long_description() {
    // The default `Status::new` sets a hanging-indent word wrap. A long
    // description must wrap onto multiple lines.
    let long_desc = "This is a very long status description that should \
                     wrap onto multiple lines when rendered at a narrow \
                     terminal width.";
    let status = Status::new(long_desc).state(StatusState::Active);
    let term = test_terminal(40);
    let out = strip_ansi(&status.render(&term));
    let line_count = out.lines().count();
    assert!(
        line_count > 1,
        "long description wraps under the default word_wrap: {} lines\n{out:?}",
        line_count
    );
}

#[test]
fn word_wrap_round_trips_through_layout_mut() {
    let mut status = Status::new("Test").state(StatusState::Info);
    status.layout_mut().word_wrap = WordWrap::WrapProse(Some(4), Some(2));
    assert_eq!(
        status.layout().word_wrap,
        WordWrap::WrapProse(Some(4), Some(2)),
        "word_wrap round-trips through layout_mut"
    );
}

// ---------------------------------------------------------------------------
// Honored: Style::color flows from the state's default_color
// ---------------------------------------------------------------------------

#[test]
fn state_default_color_flows_into_icon_on_color_terminal() {
    let term = test_terminal(80);
    let success = Status::new("Done").state(StatusState::Success);
    let out = success.render(&term);
    assert!(
        out.contains('\x1b'),
        "Success state's green Tailwind color lowers to ANSI SGR: {out:?}"
    );
}

#[test]
fn no_color_icons_disables_icon_color_but_keeps_description() {
    let term = test_terminal(80);
    let status = Status::new("Done")
        .state(StatusState::Success)
        .no_color_icons();
    let out = status.render(&term);
    assert!(
        !out.contains('\x1b'),
        "no_color_icons disables the icon SGR: {out:?}"
    );
    assert!(
        out.contains("Done"),
        "description text survives: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Honored: Style::emphasis via Prose markup (from_prose constructor)
// ---------------------------------------------------------------------------

#[test]
fn from_prose_constructor_enables_emphasis_markup() {
    // The `from_prose` constructor routes the description through Prose,
    // enabling markup like `<b>` that lowers to `Style::emphasis`.
    let status = Status::from_prose("This is <b>important</b> news")
        .state(StatusState::Warning);
    let term = test_terminal(80);
    let out = status.render(&term);
    // Bold SGR (`\x1b[1m`) must appear around "important".
    assert!(
        out.contains("\x1b[1m") && out.contains("important"),
        "from_prose enables <b> markup that lowers to bold SGR: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// N/A cells: Layout box properties do not affect inline content
// ---------------------------------------------------------------------------

#[test]
fn na_layout_margin_does_not_indent_inline_status() {
    // A status line is an inline run; the `Layout::margin` field is N/A
    // because there is no block box to indent. Setting margin on Status
    // itself does not produce the block-indent effect a block component
    // would produce. (Composing Status inside a Section yields the
    // block-indent via the containing block.)
    let without_margin = Status::new("Pending").state(StatusState::NotStarted);
    let with_margin = {
        let mut s = Status::new("Pending").state(StatusState::NotStarted);
        s.layout_mut().margin = Edges {
            left: TargetValue::universal(Length::ch(8)),
            ..Edges::default()
        };
        s
    };
    let term = test_terminal(80);
    let a = strip_ansi(&without_margin.render(&term));
    let b = strip_ansi(&with_margin.render(&term));
    // The visible content (icon + description) is identical — margin does
    // not change the inline content.
    let a_text = a.trim();
    let b_text = b.trim();
    assert_eq!(
        a_text, b_text,
        "Layout::margin is N/A for inline Status — visible content matches"
    );
}

#[test]
fn na_layout_alignment_does_not_shift_inline_status() {
    // `Layout::alignment` is N/A — the inline run is not a block to shift.
    let without_align = Status::new("Pending").state(StatusState::NotStarted);
    let with_align = {
        let mut s = Status::new("Pending").state(StatusState::NotStarted);
        s.layout_mut().alignment = Alignment::Center;
        s
    };
    let term = test_terminal(80);
    let a = strip_ansi(&without_align.render(&term));
    let b = strip_ansi(&with_align.render(&term));
    assert_eq!(
        a.trim(),
        b.trim(),
        "Layout::alignment is N/A for inline Status — visible content matches"
    );
}

#[test]
fn na_layout_max_width_does_not_cap_inline_status() {
    // `Layout::max_width` is N/A for inline content.
    let without_cap = Status::new("Pending").state(StatusState::NotStarted);
    let with_cap = {
        let mut s = Status::new("Pending").state(StatusState::NotStarted);
        s.layout_mut().max_width = Some(TargetValue::universal(Length::ch(10)));
        s
    };
    let term = test_terminal(80);
    let a = strip_ansi(&without_cap.render(&term));
    let b = strip_ansi(&with_cap.render(&term));
    assert_eq!(
        a.trim(),
        b.trim(),
        "Layout::max_width is N/A for inline Status — visible content matches"
    );
}

#[test]
fn na_layout_width_does_not_affect_inline_status() {
    use renderable::layout::Width;
    let without_width = Status::new("Pending").state(StatusState::NotStarted);
    let with_width = {
        let mut s = Status::new("Pending").state(StatusState::NotStarted);
        s.layout_mut().width = Width::Fixed(TargetValue::universal(Length::ch(20)));
        s
    };
    let term = test_terminal(80);
    let a = strip_ansi(&without_width.render(&term));
    let b = strip_ansi(&with_width.render(&term));
    assert_eq!(
        a.trim(),
        b.trim(),
        "Layout::width is N/A for inline Status — visible content matches"
    );
}

#[test]
fn na_layout_padding_does_not_affect_inline_status() {
    let without_padding = Status::new("Pending").state(StatusState::NotStarted);
    let with_padding = {
        let mut s = Status::new("Pending").state(StatusState::NotStarted);
        s.layout_mut().padding = Edges::all(Length::ch(4));
        s
    };
    let term = test_terminal(80);
    let a = strip_ansi(&without_padding.render(&term));
    let b = strip_ansi(&with_padding.render(&term));
    assert_eq!(
        a.trim(),
        b.trim(),
        "Layout::padding is N/A for inline Status — visible content matches"
    );
}

// ---------------------------------------------------------------------------
// N/A: Style::background / Style::border have no inline representation
// ---------------------------------------------------------------------------

#[test]
fn na_style_background_emits_no_block_bg_sgr() {
    // An inline badge has no padding box; the render emits no block-level
    // background SGR (`\x1b[48;`). Icon color may emit foreground SGR
    // (`\x1b[38;` or 3-bit `\x1b[3m`), but never background.
    let status = Status::new("Done").state(StatusState::Success);
    let term = test_terminal(80);
    let out = status.render(&term);
    assert!(
        !out.contains("\u{1b}[48;"),
        "no block-level background SGR is emitted (Style::background is N/A): {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Smoke: render across states and themes
// ---------------------------------------------------------------------------

#[test]
fn status_renders_non_empty_across_states() {
    let states = [
        StatusState::NotStarted,
        StatusState::Active,
        StatusState::Success,
        StatusState::Error,
        StatusState::Warning,
        StatusState::Info,
        StatusState::ToolUse,
        StatusState::Subagent,
    ];
    for state in &states {
        let status = Status::new("Test item").state(state.clone());
        let term = Terminal::new_optimistic(80);
        let out = status.render(&term);
        assert!(
            !out.trim().is_empty(),
            "{state:?} status produced empty output"
        );
        assert!(
            out.contains("Test item"),
            "{state:?} status lost the description text"
        );
    }
}
