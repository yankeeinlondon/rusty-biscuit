//! Helpers for running a single component in a dedicated terminal.
//!
//! The library supports two execution modes:
//!
//! 1. **Embedded** — the caller owns the terminal and renders the
//!    component inside a larger frame. This module is not involved.
//! 2. **Standalone** — the component owns the terminal for the
//!    duration of a single prompt. [`run_standalone`] covers this
//!    path and is what the `question` CLI is built on.
//!
//! The event loop is factored into [`drive_event_loop`] so that tests
//! can drive a widget without touching the real terminal.

use std::io::{self, Stdout, Write};

use crossterm::{
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::{Backend, CrosstermBackend},
    style::{Modifier, Style},
    text::Line,
    widgets::StatefulWidget,
};

use super::event::EventOutcome;
use super::frame::{FrameChrome, FrameChromeConfig, HeightSpec};

/// State types that can be driven to completion by [`run_standalone`].
///
/// Every component state implements this so the standalone runner can
/// extract the final value after [`EventOutcome::Submitted`] and
/// surface validation context on demand.
pub trait StandaloneState {
    /// The owned value produced on submission.
    type Value;

    /// Returns a fresh, owned copy of the component's current value.
    ///
    /// Called once after [`EventOutcome::Submitted`] is observed.
    fn value(&self) -> Self::Value;

    /// Returns the current validation error, if any.
    ///
    /// The default implementation returns `None` — components that
    /// implement submit-time validation override it.
    fn validation_error(&self) -> Option<&str> {
        None
    }

    /// Returns a one-line help hint for the active key bindings.
    ///
    /// Rendered at the bottom of the terminal in [`run_standalone`].
    /// Return an empty string to suppress the footer.
    fn help_hint(&self) -> &str {
        ""
    }
}

/// Event-handling behaviour for a component widget.
///
/// Widgets implement this alongside `StatefulWidget` so that
/// [`run_standalone`] and [`drive_event_loop`] can drive them to
/// completion.
pub trait HandleEvent: StatefulWidget {
    /// Routes a key event into the component.
    ///
    /// Returns an [`EventOutcome`] describing what happened. See
    /// [`EventOutcome`] for the full contract.
    fn handle_event(
        &self,
        state: &mut <Self as StatefulWidget>::State,
        event: KeyEvent,
    ) -> EventOutcome;
}

/// Error kind returned by [`run_standalone`] when the user presses
/// `Ctrl-C`.
///
/// Maps to `SIGINT`-style cancellation. The runner pairs this with the
/// message `"interrupted"` so downstream CLI code can branch on the
/// [`io::ErrorKind`] without parsing the message.
pub const CANCELLED_KIND: io::ErrorKind = io::ErrorKind::Interrupted;

/// Error kind returned by [`run_standalone`] when the user presses
/// `Esc` (or the component otherwise returns
/// [`EventOutcome::Cancelled`]).
///
/// Distinct from [`CANCELLED_KIND`] so that the CLI can surface a
/// different exit code for user-initiated Esc versus `Ctrl-C`.
pub const ABORTED_KIND: io::ErrorKind = io::ErrorKind::ConnectionAborted;

/// The three terminal outcomes of a single standalone event loop.
///
/// Returned by [`drive_event_loop`] and [`drive_event_loop_with_hint`].
/// [`run_standalone`] maps the variants back onto [`io::Result`] so
/// existing call sites keep working without change.
///
/// ## Variants
///
/// - [`LoopExit::Submitted`] — the component returned
///   [`EventOutcome::Submitted`]; carries the owned `value()` from the
///   component state.
/// - [`LoopExit::CtrlC`] — the user pressed `Ctrl-C`. The runner
///   intercepts this before delegating to the component.
/// - [`LoopExit::Esc`] — the component returned
///   [`EventOutcome::Cancelled`] (typically because the user pressed
///   `Esc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopExit<V> {
    /// The component submitted `value`.
    Submitted(V),
    /// The user pressed `Ctrl-C`.
    CtrlC,
    /// The user pressed `Esc` (or the component otherwise returned
    /// [`EventOutcome::Cancelled`]).
    Esc,
}

/// Drives an event loop against an existing [`Terminal`] until the
/// component returns [`EventOutcome::Submitted`] or
/// [`EventOutcome::Cancelled`], or until `Ctrl-C` is pressed.
///
/// `read_event` yields the next event (blocking). Separating the
/// event source from the loop body lets tests inject synthetic events
/// via a [`std::vec::IntoIter`] or similar.
///
/// ## Returns
///
/// `Ok(LoopExit::Submitted(value))` on submission,
/// `Ok(LoopExit::Esc)` on component-level cancellation (Esc), and
/// `Ok(LoopExit::CtrlC)` when the runner observes `Ctrl-C`. Returns
/// `Err` only for terminal I/O errors.
///
/// ## Notes
///
/// - The initial draw happens before the first event read. After that,
///   the terminal only redraws when an event is [`EventOutcome::Consumed`]
///   or when the terminal emits a resize. [`EventOutcome::Ignored`] and
///   ordinary key release events skip the redraw.
/// - Ordinary key release events (sent by crossterm on some platforms)
///   are silently skipped. Modifier-only release events are dispatched so
///   components can clear transient modifier-held state.
/// - `Ctrl-C` is mapped to [`LoopExit::CtrlC`] at the runner layer so
///   that every component honours it without needing its own binding.
pub fn drive_event_loop<C, S, B, F>(
    terminal: &mut Terminal<B>,
    component: C,
    state: &mut S,
    read_event: F,
) -> io::Result<LoopExit<S::Value>>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState,
    B: Backend,
    F: FnMut() -> io::Result<Event>,
{
    drive_event_loop_with_hint(terminal, component, state, read_event, None)
}

/// Like [`drive_event_loop`] but renders an optional help hint at the
/// bottom of the terminal area.
pub fn drive_event_loop_with_hint<C, S, B, F>(
    terminal: &mut Terminal<B>,
    component: C,
    state: &mut S,
    mut read_event: F,
    help_hint: Option<&str>,
) -> io::Result<LoopExit<S::Value>>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState,
    B: Backend,
    F: FnMut() -> io::Result<Event>,
{
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|f| {
                let area = f.area();
                let widget = component.clone();
                f.render_stateful_widget(widget, area, state);
                if let Some(hint_text) = help_hint
                    && !hint_text.is_empty()
                    && area.height > 1
                {
                    let y = area.bottom().saturating_sub(1);
                    let hint_line = Line::styled(
                        hint_text.to_string(),
                        Style::default().add_modifier(Modifier::DIM),
                    );
                    f.buffer_mut().set_line(area.x, y, &hint_line, area.width);
                }
            })?;
            needs_redraw = false;
        }

        match read_event()? {
            Event::Key(key) => {
                if !should_dispatch_key_event(&key) {
                    continue;
                }
                if is_ctrl_c(&key) {
                    return Ok(LoopExit::CtrlC);
                }
                match component.handle_event(state, key) {
                    EventOutcome::Submitted => return Ok(LoopExit::Submitted(state.value())),
                    EventOutcome::Cancelled => return Ok(LoopExit::Esc),
                    EventOutcome::Consumed => {
                        needs_redraw = true;
                    }
                    EventOutcome::Ignored => {}
                }
            }
            Event::Resize(..) => {
                needs_redraw = true;
            }
            _ => {}
        }
    }
}

/// Like [`drive_event_loop_with_hint`] but wraps the component in a
/// [`FrameChrome`] built from `chrome` on every redraw.
///
/// When `chrome.is_empty()` the rendering is identical to
/// [`drive_event_loop_with_hint`]; supplying a config with a visible
/// border or non-zero margin draws the chrome around the component.
///
/// ## Notes
///
/// The chrome is rebuilt per draw so that the inner widget is drawn
/// into the rectangle remaining after margin and border have claimed
/// their space. The help hint, when present, still renders at the
/// outer-area bottom — outside of the chrome — so it does not clash
/// with a bordered frame.
pub fn drive_event_loop_with_chrome<C, S, B, F>(
    terminal: &mut Terminal<B>,
    component: C,
    state: &mut S,
    mut read_event: F,
    help_hint: Option<&str>,
    chrome: &FrameChromeConfig,
) -> io::Result<LoopExit<S::Value>>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState,
    B: Backend,
    F: FnMut() -> io::Result<Event>,
{
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|f| {
                let area = f.area();
                let widget = component.clone();
                if chrome.is_empty() {
                    f.render_stateful_widget(widget, area, state);
                } else {
                    let frame = FrameChrome::from_config(widget, chrome);
                    f.render_stateful_widget(frame, area, state);
                }
                if let Some(hint_text) = help_hint
                    && !hint_text.is_empty()
                    && area.height > 1
                {
                    let y = area.bottom().saturating_sub(1);
                    let hint_line = Line::styled(
                        hint_text.to_string(),
                        Style::default().add_modifier(Modifier::DIM),
                    );
                    f.buffer_mut().set_line(area.x, y, &hint_line, area.width);
                }
            })?;
            needs_redraw = false;
        }

        match read_event()? {
            Event::Key(key) => {
                if !should_dispatch_key_event(&key) {
                    continue;
                }
                if is_ctrl_c(&key) {
                    return Ok(LoopExit::CtrlC);
                }
                match component.handle_event(state, key) {
                    EventOutcome::Submitted => return Ok(LoopExit::Submitted(state.value())),
                    EventOutcome::Cancelled => return Ok(LoopExit::Esc),
                    EventOutcome::Consumed => {
                        needs_redraw = true;
                    }
                    EventOutcome::Ignored => {}
                }
            }
            Event::Resize(..) => {
                needs_redraw = true;
            }
            _ => {}
        }
    }
}

/// Runs `component` in a dedicated terminal until submission or
/// cancellation.
///
/// ## Parameters
///
/// - `component` — the widget to drive. Must be cheap to clone
///   (widgets in this crate are zero-sized markers).
/// - `state` — the initial component state. Consumed by the runner.
/// - `height` — `None` runs the prompt fullscreen (alternate screen).
///   `Some(HeightSpec::Cells(n))` runs inline for `n` rows below the
///   cursor. `Some(HeightSpec::Percent(p))` queries the current
///   terminal size to translate the percentage into an absolute row
///   count.
///
/// ## Returns
///
/// The submitted value from `state.value()`.
///
/// ## Errors
///
/// Returns [`CANCELLED_KIND`] when the user pressed `Ctrl-C`, and
/// [`ABORTED_KIND`] when the user pressed `Esc`. Propagates any other
/// terminal I/O error.
pub fn run_standalone<C, S, V>(component: C, state: S, height: Option<HeightSpec>) -> io::Result<V>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState<Value = V>,
{
    run_standalone_with_chrome(component, state, height, FrameChromeConfig::default())
}

/// Runs `component` in a dedicated terminal until submission or
/// cancellation, wrapping the component in the supplied
/// [`FrameChromeConfig`].
///
/// Behaviour matches [`run_standalone`] when `chrome.is_empty()`. A
/// non-empty chrome draws a border and/or margin around the component
/// on every frame.
///
/// ## Parameters
///
/// - `height` — `None` runs the prompt fullscreen (alternate screen).
///   `Some(HeightSpec::Cells(n))` runs inline for `n` rows below the
///   cursor. `Some(HeightSpec::Percent(p))` queries the current
///   terminal size to translate the percentage into an absolute row
///   count (clamped to a floor of 3 rows so there is always room for a
///   list plus an error row).
pub fn run_standalone_with_chrome<C, S, V>(
    component: C,
    mut state: S,
    height: Option<HeightSpec>,
    chrome: FrameChromeConfig,
) -> io::Result<V>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState<Value = V>,
{
    let hint = state.help_hint().to_string();
    let fullscreen = height.is_none();
    let kbd_pushed = prepare_terminal(fullscreen)?;
    let mut guard = TerminalGuard::new(fullscreen, kbd_pushed);

    let resolved_rows = match height {
        Some(spec) => Some(resolve_height_spec(spec)?),
        None => None,
    };

    let backend = CrosstermBackend::new(io::stdout());
    let options = TerminalOptions {
        viewport: match resolved_rows {
            Some(h) => Viewport::Inline(h),
            None => Viewport::Fullscreen,
        },
    };
    let mut terminal = Terminal::with_options(backend, options)?;

    let hint_opt = if hint.is_empty() {
        None
    } else {
        Some(hint.as_str())
    };
    let loop_result = drive_event_loop_with_chrome(
        &mut terminal,
        component,
        &mut state,
        event::read,
        hint_opt,
        &chrome,
    );

    guard.dismiss();
    restore_terminal(fullscreen, kbd_pushed);

    loop_exit_to_result(loop_result?)
}

/// Maps a [`LoopExit`] onto the [`io::Result`] contract used by the
/// public [`run_standalone`] API.
///
/// Factored out so unit tests can exercise the mapping without
/// spinning up a real terminal.
fn loop_exit_to_result<V>(exit: LoopExit<V>) -> io::Result<V> {
    match exit {
        LoopExit::Submitted(v) => Ok(v),
        LoopExit::CtrlC => Err(io::Error::new(CANCELLED_KIND, "interrupted")),
        LoopExit::Esc => Err(io::Error::new(ABORTED_KIND, "cancelled")),
    }
}

/// Prepares the terminal for raw-mode TUI rendering.
///
/// Enables raw mode, enters the alternate screen when `fullscreen` is
/// true, and attempts to push kitty keyboard enhancement flags so
/// that modifier-only key events are reported.
///
/// Returns `Ok(true)` when the keyboard enhancement flags were
/// successfully pushed. The caller must pass this flag back to
/// [`restore_terminal`] so the flags are popped symmetrically.
fn prepare_terminal(fullscreen: bool) -> io::Result<bool> {
    enable_raw_mode()?;
    let mut out: Stdout = io::stdout();
    if fullscreen {
        execute!(out, EnterAlternateScreen)?;
    }
    let kbd_pushed = execute!(
        out,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
        )
    )
    .is_ok();
    out.flush().ok();
    Ok(kbd_pushed)
}

/// Restores the terminal after a standalone prompt.
///
/// Pops keyboard enhancement flags when `kbd_pushed` is true, leaves
/// the alternate screen when `fullscreen` is true, and disables raw
/// mode.
fn restore_terminal(fullscreen: bool, kbd_pushed: bool) {
    if kbd_pushed {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    if fullscreen {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
    let _ = disable_raw_mode();
}

/// Guard that calls [`restore_terminal`] on drop.
///
/// Ensures the terminal is restored even when the caller panics.
struct TerminalGuard {
    fullscreen: bool,
    kbd_pushed: bool,
    done: bool,
}

impl TerminalGuard {
    fn new(fullscreen: bool, kbd_pushed: bool) -> Self {
        Self {
            fullscreen,
            kbd_pushed,
            done: false,
        }
    }

    fn dismiss(&mut self) {
        self.done = true;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.done {
            restore_terminal(self.fullscreen, self.kbd_pushed);
        }
    }
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn should_dispatch_key_event(key: &KeyEvent) -> bool {
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => true,
        KeyEventKind::Release => matches!(key.code, KeyCode::Modifier(_)),
    }
}

/// Resolves a [`HeightSpec`] to an absolute row count.
///
/// `Cells` pass through the [`HeightSpec::resolve`] clamp against the
/// current terminal height; `Percent` queries
/// [`crossterm::terminal::size`] once so the percentage is applied to
/// the live terminal geometry.
///
/// ## Errors
///
/// Propagates any terminal I/O error reported by crossterm. When the
/// size cannot be determined (e.g. stdout is not a TTY), the caller
/// sees the underlying [`io::Error`].
fn resolve_height_spec(spec: HeightSpec) -> io::Result<u16> {
    let (_cols, rows) = crossterm::terminal::size()?;
    Ok(spec.resolve(rows))
}

#[cfg(test)]
mod tests {
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
    fn loop_exit_to_result_forwards_submitted_value() {
        let ok = loop_exit_to_result(LoopExit::Submitted("payload".to_string())).unwrap();
        assert_eq!(ok, "payload");
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
}
