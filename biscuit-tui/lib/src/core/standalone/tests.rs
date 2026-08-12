use super::inline_viewport::maybe_recompute_inline_height;
use super::terminal_lifecycle::{PrepareGuard, prepare_terminal_inner};
use super::*;
use crossterm::event::ModifierKeyCode;
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, style::Style};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Minimal widget used for Phase 1 integration tests.
///
/// Renders the current buffer contents and accumulates typed
/// characters. `Enter` submits; `Esc` cancels.
#[derive(Clone, Debug)]
struct Echo;

#[derive(Debug, Default)]
struct EchoState {
    buffer: String,
    validation_error: Option<String>,
    render_count: usize,
}

impl StatefulWidget for Echo {
    type State = EchoState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.render_count += 1;
        buf.set_string(area.x, area.y, &state.buffer, Style::default());
    }
}

impl HandleEvent for Echo {
    fn handle_event(&self, state: &mut EchoState, event: KeyEvent) -> EventOutcome {
        match event.code {
            KeyCode::Enter => EventOutcome::Submitted,
            KeyCode::Esc => EventOutcome::Cancelled,
            KeyCode::Char(c) => {
                state.buffer.push(c);
                EventOutcome::Consumed
            }
            KeyCode::Backspace => {
                state.buffer.pop();
                EventOutcome::Consumed
            }
            _ => EventOutcome::Ignored,
        }
    }
}

impl StandaloneState for EchoState {
    type Value = String;

    fn value(&self) -> Self::Value {
        self.buffer.clone()
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }
}

#[derive(Clone, Debug)]
struct ModifierProbe;

#[derive(Debug, Default)]
struct ModifierProbeState {
    badge_visible: bool,
    dispatched: Vec<KeyEvent>,
    render_count: usize,
}

impl StatefulWidget for ModifierProbe {
    type State = ModifierProbeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.render_count += 1;
        let label = if state.badge_visible {
            "badges-visible"
        } else {
            "badges-hidden"
        };
        buf.set_string(area.x, area.y, label, Style::default());
    }
}

impl HandleEvent for ModifierProbe {
    fn handle_event(&self, state: &mut ModifierProbeState, event: KeyEvent) -> EventOutcome {
        match event.code {
            KeyCode::Enter => EventOutcome::Submitted,
            KeyCode::Modifier(_) => {
                state.dispatched.push(event);
                state.badge_visible = event.kind != KeyEventKind::Release;
                EventOutcome::Consumed
            }
            _ => EventOutcome::Ignored,
        }
    }
}

impl StandaloneState for ModifierProbeState {
    type Value = bool;

    fn value(&self) -> Self::Value {
        self.badge_visible
    }
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn key_event_with_kind(code: KeyCode, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind))
}

fn modifier_event(modifier: ModifierKeyCode, kind: KeyEventKind) -> Event {
    key_event_with_kind(KeyCode::Modifier(modifier), kind)
}

fn run(events: Vec<Event>) -> io::Result<LoopExit<String>> {
    let backend = TestBackend::new(20, 3);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = EchoState::default();
    let mut iter = events.into_iter();
    drive_event_loop(&mut terminal, Echo, &mut state, || {
        iter.next()
            .ok_or_else(|| io::Error::other("no more events"))
    })
}

fn run_capturing_state(events: Vec<Event>) -> (io::Result<LoopExit<String>>, EchoState) {
    let backend = TestBackend::new(20, 3);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = EchoState::default();
    let mut iter = events.into_iter();
    let result = drive_event_loop(&mut terminal, Echo, &mut state, || {
        iter.next()
            .ok_or_else(|| io::Error::other("no more events"))
    });
    (result, state)
}

#[test]
fn submits_on_enter() {
    let events = vec![
        key_event(KeyCode::Char('h')),
        key_event(KeyCode::Char('i')),
        key_event(KeyCode::Enter),
    ];
    let result = run(events).expect("drive loop");
    assert_eq!(result, LoopExit::Submitted("hi".to_string()));
}

#[test]
fn cancels_on_esc() {
    let events = vec![key_event(KeyCode::Char('x')), key_event(KeyCode::Esc)];
    let result = run(events).expect("drive loop");
    assert_eq!(result, LoopExit::Esc);
}

#[test]
fn cancels_on_ctrl_c() {
    let events = vec![Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    ))];
    let result = run(events).expect("drive loop");
    assert_eq!(result, LoopExit::CtrlC);
}

#[test]
fn loop_exit_distinguishes_esc_from_ctrl_c() {
    let esc = run(vec![key_event(KeyCode::Esc)]).expect("drive loop");
    let ctrl_c = run(vec![Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    ))])
    .expect("drive loop");
    assert_eq!(esc, LoopExit::Esc);
    assert_eq!(ctrl_c, LoopExit::CtrlC);
    assert_ne!(esc, ctrl_c);
}

#[test]
fn ignored_events_do_not_exit_the_loop() {
    let events = vec![
        Event::Resize(40, 10),
        key_event(KeyCode::F(5)),
        key_event(KeyCode::Char('a')),
        key_event(KeyCode::Enter),
    ];
    let result = run(events).expect("drive loop");
    assert_eq!(result, LoopExit::Submitted("a".to_string()));
}

#[test]
fn key_release_events_are_skipped() {
    let release = key_event_with_kind(KeyCode::Char('q'), KeyEventKind::Release);
    let events = vec![
        release,
        key_event(KeyCode::Char('b')),
        key_event(KeyCode::Enter),
    ];
    let result = run(events).expect("drive loop");
    assert_eq!(result, LoopExit::Submitted("b".to_string()));
}

#[test]
fn modifier_release_events_reach_standalone_loop_and_redraw_hidden_badges() {
    let backend = TestBackend::new(24, 3);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = ModifierProbeState::default();
    let events = vec![
        modifier_event(ModifierKeyCode::LeftControl, KeyEventKind::Press),
        modifier_event(ModifierKeyCode::LeftControl, KeyEventKind::Release),
        key_event(KeyCode::Enter),
    ];
    let mut iter = events.into_iter();
    let result = drive_event_loop(&mut terminal, ModifierProbe, &mut state, || {
        iter.next()
            .ok_or_else(|| io::Error::other("no more events"))
    });

    assert_eq!(result.expect("drive loop"), LoopExit::Submitted(false));
    assert_eq!(state.dispatched.len(), 2);
    assert_eq!(state.dispatched[0].kind, KeyEventKind::Press);
    assert_eq!(state.dispatched[1].kind, KeyEventKind::Release);
    assert_eq!(state.render_count, 3);
    assert_eq!(
        terminal.backend().buffer()[(0, 0)].symbol(),
        "b",
        "final redraw should render badges-hidden after modifier release",
    );
}

#[test]
fn modifier_release_events_reach_chrome_loop_and_redraw_hidden_badges() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};

    let backend = TestBackend::new(28, 5);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = ModifierProbeState::default();
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        border_label: Some("Hotkeys".to_string()),
        ..Default::default()
    };
    let events = vec![
        modifier_event(ModifierKeyCode::LeftAlt, KeyEventKind::Press),
        modifier_event(ModifierKeyCode::LeftAlt, KeyEventKind::Release),
        key_event(KeyCode::Enter),
    ];
    let mut iter = events.into_iter();
    let result = drive_event_loop_with_chrome(
        &mut terminal,
        ModifierProbe,
        &mut state,
        || {
            iter.next()
                .ok_or_else(|| io::Error::other("no more events"))
        },
        None,
        &chrome,
    );

    assert_eq!(result.expect("drive loop"), LoopExit::Submitted(false));
    assert_eq!(state.dispatched.len(), 2);
    assert_eq!(state.dispatched[0].kind, KeyEventKind::Press);
    assert_eq!(state.dispatched[1].kind, KeyEventKind::Release);
    assert_eq!(state.render_count, 3);

    let row_text = |y: u16| -> String {
        let buf = terminal.backend().buffer();
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row
    };
    assert!(
        (1..4).any(|y| row_text(y).contains("badges-hidden")),
        "final chrome redraw should render badges-hidden after modifier release",
    );
}

#[test]
fn ignored_events_do_not_trigger_a_redraw() {
    let events = vec![
        key_event(KeyCode::F(5)),
        key_event(KeyCode::F(6)),
        key_event(KeyCode::Enter),
    ];
    let (result, state) = run_capturing_state(events);
    assert_eq!(
        result.expect("drive loop"),
        LoopExit::Submitted(String::new())
    );
    // Initial draw (1). F5/F6 are Ignored — no extra draws. Enter
    // submits and exits the loop without a further draw.
    assert_eq!(state.render_count, 1);
}

#[test]
fn consumed_events_trigger_a_redraw() {
    let events = vec![
        key_event(KeyCode::Char('a')),
        key_event(KeyCode::Char('b')),
        key_event(KeyCode::Enter),
    ];
    let (result, state) = run_capturing_state(events);
    assert_eq!(
        result.expect("drive loop"),
        LoopExit::Submitted("ab".to_string())
    );
    // Initial draw + 2 Consumed events = 3 draws.
    assert_eq!(state.render_count, 3);
}

#[test]
fn resize_events_trigger_a_redraw() {
    let events = vec![Event::Resize(40, 10), key_event(KeyCode::Enter)];
    let (result, state) = run_capturing_state(events);
    assert_eq!(
        result.expect("drive loop"),
        LoopExit::Submitted(String::new())
    );
    // Initial draw + resize redraw = 2 draws.
    assert_eq!(state.render_count, 2);
}

#[test]
fn standalone_state_default_validation_error_is_none() {
    struct Minimal;
    impl StandaloneState for Minimal {
        type Value = ();
        fn value(&self) -> Self::Value {}
    }
    assert!(Minimal.validation_error().is_none());
}

#[test]
fn standalone_runner_smoke_test_with_echo_widget() {
    let backend = TestBackend::new(20, 3);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = EchoState::default();
    let events = vec![
        key_event(KeyCode::Char('h')),
        key_event(KeyCode::Char('i')),
        key_event(KeyCode::Enter),
    ];
    let mut iter = events.into_iter();
    let result = drive_event_loop(&mut terminal, Echo, &mut state, || {
        iter.next()
            .ok_or_else(|| io::Error::other("no more events"))
    });
    assert_eq!(
        result.expect("drive loop"),
        LoopExit::Submitted("hi".to_string())
    );
    assert_eq!(state.render_count, 3);
}

#[test]
fn run_standalone_returns_aborted_kind_on_esc() {
    let err = loop_exit_to_result::<()>(LoopExit::Esc).unwrap_err();
    assert_eq!(err.kind(), ABORTED_KIND);
}

#[test]
fn run_standalone_returns_cancelled_kind_on_ctrl_c() {
    let err = loop_exit_to_result::<()>(LoopExit::CtrlC).unwrap_err();
    assert_eq!(err.kind(), CANCELLED_KIND);
}

#[test]
fn run_standalone_aborted_and_cancelled_kinds_are_distinct() {
    assert_ne!(CANCELLED_KIND, ABORTED_KIND);
}

#[test]
fn maybe_recompute_inline_height_returns_none_for_cells() {
    // Cells variant is treated as a static cap; ratatui's autoresize
    // already clamps the inline viewport against the live terminal
    // height, so the recompute helper must not signal a recreation.
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Cells(15), 30, 15),
        None,
    );
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Cells(15), 5, 15),
        None,
    );
}

#[test]
fn maybe_recompute_inline_height_returns_none_when_percent_unchanged() {
    // 50% of 40 rows is 20 (matches current_h) — no recreation.
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Percent(50), 40, 20),
        None,
    );
}

#[test]
fn maybe_recompute_inline_height_returns_new_height_when_percent_grew() {
    // Terminal grew from 40 → 60 rows; 50% of 60 = 30, differs from 20.
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Percent(50), 60, 20),
        Some(30),
    );
}

#[test]
fn maybe_recompute_inline_height_returns_new_height_when_percent_shrank() {
    // Terminal shrank from 40 → 20 rows; 50% of 20 = 10, differs from 20.
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Percent(50), 20, 20),
        Some(10),
    );
}

#[test]
fn maybe_recompute_inline_height_respects_percent_floor() {
    // Percent floor is 3 rows; tiny terminals collapse to that floor.
    // current_h was already 3, so no signal — even though the math
    // would otherwise yield 0.
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Percent(10), 4, 3),
        None,
    );
    // From a larger height, shrinking to the floor still emits a
    // recreation signal.
    assert_eq!(
        maybe_recompute_inline_height(HeightSpec::Percent(10), 4, 10),
        Some(3),
    );
}

#[test]
fn loop_exit_to_result_forwards_submitted_value() {
    let ok = loop_exit_to_result(LoopExit::Submitted("payload".to_string())).unwrap();
    assert_eq!(ok, "payload");
}

#[test]
fn dispatcher_passes_ctrl_space_release_through() {
    // Required so the press-and-hold sticky-toggle UX in the
    // choose components can clear the sticky mode on chord
    // release. The previous filter blocked all `Char` releases
    // and the chord clear never fired.
    let release = KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    assert!(should_dispatch_key_event(&release));
}

#[test]
fn dispatcher_passes_legacy_nul_ctrl_space_release_through() {
    // The non-kitty Ctrl+Space encoding (NUL byte) on release.
    let release = KeyEvent {
        code: KeyCode::Char('\0'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    assert!(should_dispatch_key_event(&release));
}

#[test]
fn dispatcher_passes_alt_space_release_through() {
    let release = KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    assert!(should_dispatch_key_event(&release));
}

#[test]
fn dispatcher_filters_unrelated_char_releases() {
    // Releases of arbitrary characters MUST still be filtered —
    // letting them through could fire bindings (Esc, Enter, etc.)
    // on key release, which components don't expect.
    let release = KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    assert!(!should_dispatch_key_event(&release));
}

#[test]
fn dispatcher_filters_esc_release() {
    let release = KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    assert!(!should_dispatch_key_event(&release));
}

#[test]
fn drive_event_loop_with_chrome_empty_renders_into_full_area() {
    use crate::core::frame::FrameChromeConfig;

    let backend = TestBackend::new(20, 4);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = EchoState::default();
    let events = vec![
        key_event(KeyCode::Char('h')),
        key_event(KeyCode::Char('i')),
        key_event(KeyCode::Enter),
    ];
    let mut iter = events.into_iter();
    let result = drive_event_loop_with_chrome(
        &mut terminal,
        Echo,
        &mut state,
        || {
            iter.next()
                .ok_or_else(|| io::Error::other("no more events"))
        },
        None,
        &FrameChromeConfig::default(),
    );
    assert_eq!(
        result.expect("drive loop"),
        LoopExit::Submitted("hi".to_string())
    );
}

#[test]
fn drive_event_loop_with_chrome_draws_border_around_inner() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};

    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = EchoState::default();
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        border_label: Some("T".to_string()),
        ..Default::default()
    };
    let events = vec![key_event(KeyCode::Enter)];
    let mut iter = events.into_iter();
    let _result = drive_event_loop_with_chrome(
        &mut terminal,
        Echo,
        &mut state,
        || {
            iter.next()
                .ok_or_else(|| io::Error::other("no more events"))
        },
        None,
        &chrome,
    );

    let buf = terminal.backend().buffer().clone();
    // Top-left and bottom-right corners are border glyphs.
    let top_left = buf[(0, 0)].symbol().to_string();
    let bottom_right = buf[(7, 3)].symbol().to_string();
    assert_ne!(top_left, " ");
    assert_ne!(bottom_right, " ");
}

#[test]
fn drive_event_loop_with_chrome_preserves_search_prompt_inside_border() {
    use crate::components::{ChoiceInput, ChoiceOption, ChooseOne, ChooseOneState};
    use crate::core::frame::{BorderStyle, FrameChromeConfig};

    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let input = ChoiceInput::new("colour", "Pick a colour")
        .with_filter_enabled(true)
        .with_options(vec![
            ChoiceOption::new("r", "Red", "red".to_string()),
            ChoiceOption::new("g", "Green", "green".to_string()),
            ChoiceOption::new("b", "Blue", "blue".to_string()),
        ]);
    let mut state = ChooseOneState::new(input);
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        border_label: Some("Pick".to_string()),
        ..Default::default()
    };
    // Seed the filter with pattern "re" by routing key events through
    // the same event iterator the existing chrome tests use, then
    // submit with Enter so the loop exits cleanly.
    let events = vec![
        key_event(KeyCode::Char('r')),
        key_event(KeyCode::Char('e')),
        key_event(KeyCode::Enter),
    ];
    let mut iter = events.into_iter();
    let _result = drive_event_loop_with_chrome(
        &mut terminal,
        ChooseOne::<String>::new(),
        &mut state,
        || {
            iter.next()
                .ok_or_else(|| io::Error::other("no more events"))
        },
        None,
        &chrome,
    );

    let buf = terminal.backend().buffer().clone();
    // (a) Border glyphs in the expected corners.
    let top_left = buf[(0, 0)].symbol().to_string();
    let bottom_right = buf[(29, 7)].symbol().to_string();
    assert_ne!(
        top_left, " ",
        "top-left corner should contain a border glyph"
    );
    assert_ne!(
        bottom_right, " ",
        "bottom-right corner should contain a border glyph"
    );

    // Helper: read a row from the buffer as a single string.
    let row_text = |y: u16| -> String {
        let mut s = String::new();
        for x in buf.area.left()..buf.area.right() {
            s.push_str(buf[(x, y)].symbol());
        }
        s
    };

    // (b) The search prompt row is drawn inside the border. Scan
    // interior rows (1..height-1) for the theme's default search
    // indicator ("/ ") followed by the seeded pattern "re".
    let mut found_prompt = false;
    for y in 1..7 {
        let row = row_text(y);
        if row.contains("/ re") {
            found_prompt = true;
            break;
        }
    }
    assert!(
        found_prompt,
        "expected a row inside the border to contain '/ re'; buffer rows:\n{}",
        (0..8).map(row_text).collect::<Vec<_>>().join("\n"),
    );

    // (c) At least one row inside the border contains a non-space
    // char from "Red" or "Green" — proves the filter pattern routed
    // through to the visible list.
    let mut found_match = false;
    for y in 1..7 {
        let row = row_text(y);
        if row.contains("Red") || row.contains("Green") {
            found_match = true;
            break;
        }
    }
    assert!(
        found_match,
        "expected a visible label (Red or Green) inside the border",
    );
}

// --- Help-hint placement ----------------------------------------

#[test]
fn prepare_chrome_for_hint_folds_hint_into_bottom_label_when_chrome_has_bottom() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        ..Default::default()
    };
    let (effective, needs_overlay) = prepare_chrome_for_hint(&chrome, Some("Enter=Submit"));
    assert_eq!(effective.bottom_label.as_deref(), Some("Enter=Submit"));
    assert!(
        !needs_overlay,
        "with bottom border the hint must be hosted by the chrome, not overlaid",
    );
}

#[test]
fn prepare_chrome_for_hint_falls_back_to_overlay_when_no_bottom_border() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};
    let chrome = FrameChromeConfig {
        border: BorderStyle::Top,
        ..Default::default()
    };
    let (effective, needs_overlay) = prepare_chrome_for_hint(&chrome, Some("Enter=Submit"));
    assert!(
        effective.bottom_label.is_none(),
        "no bottom border → bottom_label stays empty",
    );
    assert!(
        needs_overlay,
        "no bottom border → caller must still draw an overlay row",
    );
}

#[test]
fn prepare_chrome_for_hint_preserves_caller_supplied_bottom_label() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        bottom_label: Some("Custom".to_string()),
        ..Default::default()
    };
    let (effective, needs_overlay) = prepare_chrome_for_hint(&chrome, Some("Enter=Submit"));
    assert_eq!(
        effective.bottom_label.as_deref(),
        Some("Custom"),
        "caller-supplied bottom_label wins over the runner-injected hint",
    );
    // Hint still needs somewhere to go — caller draws it as overlay.
    assert!(needs_overlay);
}

#[test]
fn prepare_chrome_for_hint_skips_when_hint_is_empty() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        ..Default::default()
    };
    let (effective, needs_overlay) = prepare_chrome_for_hint(&chrome, Some(""));
    assert!(effective.bottom_label.is_none());
    assert!(!needs_overlay);
    let (effective, needs_overlay) = prepare_chrome_for_hint(&chrome, None);
    assert!(effective.bottom_label.is_none());
    assert!(!needs_overlay);
}

#[test]
fn hint_horizontal_extent_skips_left_and_right_borders() {
    use crate::core::frame::{BorderStyle, FrameChromeConfig};
    let chrome = FrameChromeConfig {
        border: BorderStyle::Vertical,
        ..Default::default()
    };
    let area = Rect::new(0, 0, 20, 5);
    let (x, width) = hint_horizontal_extent(&chrome, area);
    assert_eq!(x, 1, "hint must start past the left border glyph");
    assert_eq!(
        width, 18,
        "hint width should be area.width minus left+right border insets",
    );
}

#[test]
fn hint_horizontal_extent_uses_full_width_without_borders() {
    let chrome = FrameChromeConfig::default();
    let area = Rect::new(2, 4, 30, 6);
    let (x, width) = hint_horizontal_extent(&chrome, area);
    assert_eq!(x, area.x);
    assert_eq!(width, area.width);
}

#[test]
fn drive_event_loop_with_chrome_renders_help_hint_inside_bottom_border() {
    // Regression test for the "Enter=Submit Esc=Cancel pressed too
    // far to the left so the corners are no longer round" bug.
    // With a Rounded chrome and a non-empty hint, the hint must be
    // rendered on the bottom border row, leaving the corner glyphs
    // intact at column 0 and column area.right - 1.
    use crate::core::frame::{BorderStyle, FrameChromeConfig};
    let backend = TestBackend::new(30, 5);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    let mut state = EchoState::default();
    let chrome = FrameChromeConfig {
        border: BorderStyle::Rounded,
        ..Default::default()
    };
    let events = vec![key_event(KeyCode::Enter)];
    let mut iter = events.into_iter();
    let _ = drive_event_loop_with_chrome(
        &mut terminal,
        Echo,
        &mut state,
        || {
            iter.next()
                .ok_or_else(|| io::Error::other("no more events"))
        },
        Some("Enter=Submit  Esc=Cancel"),
        &chrome,
    );

    let buf = terminal.backend().buffer().clone();
    let last_row = (0..30)
        .map(|x| buf[(x, 4)].symbol().to_string())
        .collect::<String>();
    assert!(
        last_row.contains("Enter=Submit"),
        "hint should appear on the bottom border row; got {last_row:?}",
    );
    // Both corner glyphs must survive; the original bug overwrote
    // the bottom-left rounded glyph with the first character of
    // the hint string.
    assert_ne!(buf[(0, 4)].symbol(), " ", "bottom-left corner clobbered");
    assert_ne!(buf[(29, 4)].symbol(), " ", "bottom-right corner clobbered",);
    // Specifically: the bottom-left corner should be a rounded
    // border glyph — `╰` for `BorderType::Rounded`. We don't assert
    // the exact character (theme could change), but it must be
    // non-alphanumeric (i.e. not the 'E' from "Enter=Submit").
    let corner = buf[(0, 4)].symbol();
    assert!(
        !corner.chars().any(|c| c.is_ascii_alphanumeric()),
        "expected a border glyph at the bottom-left corner; got {corner:?}",
    );
}

// --- F1: transactional terminal preparation ----------------------------

/// Counting teardown closures shared between a `PrepareGuard`/test run
/// and its assertions, mirroring how production wires in the crossterm
/// calls but without touching the real terminal.
fn counting_teardowns() -> (
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
    impl FnMut(),
    impl FnMut(),
) {
    let disable = Arc::new(AtomicUsize::new(0));
    let leave = Arc::new(AtomicUsize::new(0));
    let disable_for_closure = disable.clone();
    let leave_for_closure = leave.clone();
    let leave_fn = move || {
        leave_for_closure.fetch_add(1, Ordering::SeqCst);
    };
    let disable_fn = move || {
        disable_for_closure.fetch_add(1, Ordering::SeqCst);
    };
    (disable, leave, leave_fn, disable_fn)
}

#[test]
fn prepare_terminal_unwinds_raw_mode_when_alt_screen_step_fails() {
    // Regression test for F1: if EnterAlternateScreen fails after raw
    // mode is enabled, raw mode MUST be disabled and LeaveAlternateScreen
    // MUST NOT be emitted (the screen was never entered). This test
    // fails if the PrepareGuard is removed — the disable counter stays
    // at zero, proving the guard is load-bearing.
    let (disable_calls, leave_calls, leave_fn, disable_fn) = counting_teardowns();

    let result = prepare_terminal_inner(
        true,
        || Ok(()),
        |_out| Err(io::Error::other("injected alt-screen failure")),
        |_out| true,
        leave_fn,
        disable_fn,
    );

    assert!(
        result.is_err(),
        "faulted alt-screen step must propagate the error",
    );
    assert_eq!(
        disable_calls.load(Ordering::SeqCst),
        1,
        "raw mode must be disabled when prepare fails after enable_raw_mode",
    );
    assert_eq!(
        leave_calls.load(Ordering::SeqCst),
        0,
        "no LeaveAlternateScreen may be emitted for a screen never entered",
    );
}

#[test]
fn prepare_terminal_happy_path_returns_kbd_flag_without_teardown() {
    // F1 success contract: the guard is dismissed on the happy path so
    // the caller's TerminalGuard owns teardown; kbd_pushed flows through
    // byte-for-byte unchanged. Mirrors the existing
    // `prepare_terminal(true)` happy path.
    let (disable_calls, leave_calls, leave_fn, disable_fn) = counting_teardowns();

    let kbd_pushed = prepare_terminal_inner(
        true,
        || Ok(()),
        |_out| Ok(()),
        |_out| true,
        leave_fn,
        disable_fn,
    )
    .expect("happy path");

    assert!(
        kbd_pushed,
        "kbd_pushed flag must flow through unchanged on the happy path",
    );
    assert_eq!(
        disable_calls.load(Ordering::SeqCst),
        0,
        "success dismisses the guard; the caller's TerminalGuard owns teardown",
    );
    assert_eq!(leave_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn prepare_terminal_inline_happy_path_propagates_kbd_flag_false() {
    // Non-fullscreen path never enters the alt screen; a `false`
    // kbd-push result must propagate verbatim (TerminalGuard::new still
    // receives the same values as before F1).
    let kbd_pushed = prepare_terminal_inner(
        false,
        || Ok(()),
        |_out| Ok(()),
        |_out| false,
        || {},
        || {},
    )
    .expect("happy path");
    assert!(!kbd_pushed);
}

#[test]
fn prepare_guard_drop_leaves_alt_screen_when_entered() {
    // Documents the guard contract for a future fallible step added
    // between alt-screen entry and dismiss: if alt-screen was marked
    // entered and the guard is NOT dismissed, Drop leaves the alt
    // screen AND disables raw mode.
    let (disable_calls, leave_calls, leave_fn, disable_fn) = counting_teardowns();
    {
        let mut guard = PrepareGuard::arm(leave_fn, disable_fn);
        guard.note_alt_screen_entered();
    }
    assert_eq!(leave_calls.load(Ordering::SeqCst), 1);
    assert_eq!(disable_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn prepare_guard_dismiss_suppresses_all_teardown() {
    // On the success path the guard is dismissed, so neither teardown
    // runs even when the alt screen was entered.
    let (disable_calls, leave_calls, leave_fn, disable_fn) = counting_teardowns();
    {
        let mut guard = PrepareGuard::arm(leave_fn, disable_fn);
        guard.note_alt_screen_entered();
        guard.dismiss();
    }
    assert_eq!(leave_calls.load(Ordering::SeqCst), 0);
    assert_eq!(disable_calls.load(Ordering::SeqCst), 0);
}

// --- F2: Windows console redirect (CI-only; macOS host compiles-skip) ---
//
// The reviewing host is macOS, so these run only on the Windows CI runner.
// They prove the handle strategy the Windows `StdoutTtyRedirect` relies on:
// CONOUT$ is openable as a real console, and SetStdHandle is honored by
// GetStdHandle (the function Rust's io::stdout() and crossterm use to
// resolve the active output handle dynamically), so redirecting the std
// handle really reroutes stdout writes onto the console.

#[cfg(windows)]
mod windows_redirect {
    use super::*;
    use crate::core::standalone::terminal_lifecycle::CONOUT;
    use std::io::{IsTerminal, Read, Write};
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, GetFileType, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_CHAR, FILE_TYPE_PIPE,
        OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetStdHandle, STD_OUTPUT_HANDLE,
    };
    use windows_sys::Win32::System::Pipes::CreatePipe;

    fn open_conout() -> windows_sys::Win32::Foundation::HANDLE {
        // SAFETY: CONOUT is a static NUL-terminated UTF-16 device path;
        // access mode and disposition are the documented console values.
        unsafe {
            CreateFileW(
                CONOUT.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                core::ptr::null(),
                OPEN_EXISTING,
                0,
                core::ptr::null_mut(),
            )
        }
    }

    #[test]
    fn conout_is_openable_console_handle() {
        // Prove the core of the redirect: CONOUT$ opens even when this
        // process's own std handles are whatever the CI runner supplied,
        // and the result is a genuine console screen buffer (GetConsoleMode
        // succeeds only for console handles — the robust check).
        let conout = open_conout();
        assert_ne!(
            conout, INVALID_HANDLE_VALUE,
            "CreateFileW(CONOUT$) must succeed under a CI console",
        );
        let mut mode = 0u32;
        // SAFETY: conout is a valid open handle from CreateFileW; mode is a
        // valid out-pointer.
        let is_console = unsafe { GetConsoleMode(conout, &mut mode) } != 0;
        assert!(
            is_console,
            "CONOUT$ handle must be a real console screen buffer",
        );
        // SAFETY: conout is the handle we opened and have not closed yet.
        unsafe { CloseHandle(conout) };
    }

    #[test]
    fn set_std_handle_round_trip_restores_original_stdout() {
        // Prove SetStdHandle is honored by GetStdHandle — the exact
        // mechanism the redirect relies on. crossterm's output writes and
        // Rust's io::stdout() resolve STD_OUTPUT_HANDLE dynamically via
        // GetStdHandle, so a save→redirect→restore cycle reroutes stdout
        // and then puts the original handle back exactly.
        // SAFETY: STD_OUTPUT_HANDLE is a valid std-handle identifier.
        let original = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

        let conout = open_conout();
        assert_ne!(conout, INVALID_HANDLE_VALUE);

        // SAFETY: STD_OUTPUT_HANDLE is valid; conout is open.
        assert_ne!(
            unsafe { SetStdHandle(STD_OUTPUT_HANDLE, conout) },
            0,
            "SetStdHandle must succeed",
        );
        // SAFETY: STD_OUTPUT_HANDLE is valid.
        let after_redirect = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        assert_eq!(
            after_redirect, conout,
            "GetStdHandle must reflect the redirected handle — this is what \
             makes io::stdout() reroute to CONOUT$",
        );

        // SAFETY: restore the original handle.
        assert_ne!(unsafe { SetStdHandle(STD_OUTPUT_HANDLE, original) }, 0);
        // SAFETY: STD_OUTPUT_HANDLE is valid.
        let restored = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        assert_eq!(
            restored, original,
            "restore must put the original stdout handle back exactly",
        );

        // SAFETY: conout is our owned handle; std handle already restored.
        unsafe { CloseHandle(conout) };
    }

    #[test]
    fn redirect_guard_is_inactive_when_stdout_is_a_console() {
        // Gating: when stdout is already a console (the normal CI cargo-test
        // shape), activate_if_piped returns an inactive guard and Drop is a
        // clean no-op that leaves the stdout handle untouched. This is the
        // Windows mirror of the Unix no-op-when-tty contract.
        if !io::stdout().is_terminal() {
            // stdout captured in this particular run — the active path is
            // covered by the handle-strategy tests above; skip the gating
            // assertion rather than spawning a console-less edge case.
            return;
        }
        // SAFETY: STD_OUTPUT_HANDLE is valid.
        let before = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        {
            let _guard = StdoutTtyRedirect::activate_if_piped();
        }
        // SAFETY: STD_OUTPUT_HANDLE is valid.
        let after = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        assert_eq!(
            before, after,
            "inactive guard must not perturb the stdout handle",
        );
    }

    #[test]
    fn active_redirect_routes_stdout_to_console_and_pipe_gets_only_submitted_value() {
        // The user-facing F2 contract, exercised in-process and
        // deterministically: synthesize the `FOO=$(question ...)` shape by
        // pointing STD_OUTPUT_HANDLE at a pipe while stderr stays a console,
        // then prove that after StdoutTtyRedirect::activate_if_piped() the
        // process's stdout writes land on the console (CONOUT$), and the
        // captured pipe receives ONLY the value the CLI emits *after* the
        // redirect drops — never the prompt's TUI bytes. A subprocess is not
        // needed: SetStdHandle alone reproduces the captured-stdout shape
        // because Rust's io::stdout() resolves the std handle dynamically.
        const PROMPT_SENTINEL: &str = "<<PROMPT-TUI-BYTES-MUST-NOT-LEAK>>";
        const SUBMITTED_SENTINEL: &str = "<<SUBMITTED-VALUE-MUST-REACH-PIPE>>";

        // The active path requires stderr to be a real console. Under nextest
        // stderr is captured, so skip rather than assert against a shape this
        // run cannot produce — mirrors the gating test above.
        if !io::stderr().is_terminal() {
            return;
        }

        // SAFETY: STD_OUTPUT_HANDLE is a valid std-handle identifier; the
        // returned handle is borrowed (never closed here).
        let original_stdout = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

        let mut read_end: HANDLE = core::ptr::null_mut();
        let mut write_end: HANDLE = core::ptr::null_mut();
        // SAFETY: both out-pointers are valid; a null SECURITY_ATTRIBUTES and
        // 0 size request the default anonymous-pipe behavior. Return value is
        // checked before either handle is used.
        let created = unsafe {
            CreatePipe(
                &mut read_end,
                &mut write_end,
                core::ptr::null(),
                0,
            )
        };
        assert_ne!(created, 0, "CreatePipe must succeed");

        // Point the process's stdout at the pipe's write end. This is what
        // makes io::stdout().is_terminal() return false inside
        // activate_if_piped, synthesizing the captured-stdout shape.
        // SAFETY: STD_OUTPUT_HANDLE is valid; write_end is the open pipe
        // handle from CreatePipe.
        assert_ne!(
            unsafe { SetStdHandle(STD_OUTPUT_HANDLE, write_end) },
            0,
            "SetStdHandle(pipe) must succeed",
        );

        // SAFETY: handles just established are valid for GetFileType.
        assert_eq!(
            unsafe { GetFileType(GetStdHandle(STD_OUTPUT_HANDLE)) },
            FILE_TYPE_PIPE,
            "precondition: stdout now resolves to the pipe",
        );

        {
            let guard = StdoutTtyRedirect::activate_if_piped();

            // Behavioral proof of the redirect: with stdout=pipe and
            // stderr=console, the guard must have rerouted stdout to CONOUT$.
            // FILE_TYPE_CHAR is the console; FILE_TYPE_PIPE would mean writes
            // still hit the captured pipe (the bug F2 fixes).
            // SAFETY: STD_OUTPUT_HANDLE is valid post-activation.
            assert_eq!(
                unsafe { GetFileType(GetStdHandle(STD_OUTPUT_HANDLE)) },
                FILE_TYPE_CHAR,
                "active redirect must point stdout at the console, not the pipe",
            );

            // Emit the "prompt" while the guard is active. It goes to the
            // console (acceptable) and must NOT reach the captured pipe.
            let mut out = io::stdout();
            let _ = out.write_all(PROMPT_SENTINEL.as_bytes());
            let _ = out.flush();

            drop(guard);

            // Drop restores the captured stdout exactly, so subsequent writes
            // reach the pipe again — this is what lets the CLI's result
            // println! land in the caller's `$(...)` capture.
            // SAFETY: STD_OUTPUT_HANDLE is valid post-restore.
            assert_eq!(
                unsafe { GetStdHandle(STD_OUTPUT_HANDLE) },
                write_end,
                "drop must restore the handle that was current at activation",
            );
            // SAFETY: STD_OUTPUT_HANDLE is valid post-restore.
            assert_eq!(
                unsafe { GetFileType(GetStdHandle(STD_OUTPUT_HANDLE)) },
                FILE_TYPE_PIPE,
                "post-drop stdout must resolve to the pipe again",
            );
        }

        // The CLI prints the submitted value after the redirect drops; it
        // must reach the captured pipe.
        let mut out = io::stdout();
        let _ = out.write_all(SUBMITTED_SENTINEL.as_bytes());
        let _ = out.flush();

        // Restore the real stdout before reading, then close the write end so
        // the read does not block waiting for more data.
        // SAFETY: STD_OUTPUT_HANDLE is valid; original_stdout was captured
        // from GetStdHandle at entry. It is restored, never closed (borrowed).
        unsafe { SetStdHandle(STD_OUTPUT_HANDLE, original_stdout) };
        // SAFETY: write_end is the pipe handle we created and have not closed;
        // closing it once flushes EOF to the read end.
        unsafe { CloseHandle(write_end) };

        // Drain the pipe via a std File wrapper, which owns and closes the
        // read end on drop (so we do not CloseHandle(read_end) ourselves).
        // SAFETY: read_end is the valid, still-open pipe read handle from
        // CreatePipe; ownership is transferred to the File exactly once.
        let mut reader = unsafe { std::fs::File::from_raw_handle(read_end as _) };
        let mut captured = Vec::new();
        let _ = reader.read_to_end(&mut captured);
        let captured = String::from_utf8_lossy(&captured);

        assert!(
            captured.contains(SUBMITTED_SENTINEL),
            "captured pipe must receive the submitted value emitted after drop",
        );
        assert!(
            !captured.contains(PROMPT_SENTINEL),
            "captured pipe must NOT receive the prompt's TUI bytes (they went to the console)",
        );
    }
}
