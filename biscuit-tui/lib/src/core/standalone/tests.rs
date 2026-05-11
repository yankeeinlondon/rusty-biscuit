use super::inline_viewport::maybe_recompute_inline_height;
use super::*;
use crossterm::event::ModifierKeyCode;
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, style::Style};

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
