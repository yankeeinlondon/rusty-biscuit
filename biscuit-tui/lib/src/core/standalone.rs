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
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::{Backend, CrosstermBackend},
    widgets::StatefulWidget,
};

use super::event::EventOutcome;

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

/// Error returned by [`run_standalone`] when the user cancels.
///
/// The runner surfaces cancellation through [`std::io::Error`] with
/// [`std::io::ErrorKind::Interrupted`] so callers can match on the
/// kind without pulling in a custom error enum.
pub const CANCELLED_KIND: io::ErrorKind = io::ErrorKind::Interrupted;

/// Drives an event loop against an existing [`Terminal`] until the
/// component returns [`EventOutcome::Submitted`] or
/// [`EventOutcome::Cancelled`].
///
/// `read_event` yields the next event (blocking). Separating the
/// event source from the loop body lets tests inject synthetic events
/// via a [`std::vec::IntoIter`] or similar.
///
/// ## Returns
///
/// - `Ok(Some(value))` — the user submitted. The value comes from
///   `state.value()`.
/// - `Ok(None)` — the user cancelled.
/// - `Err(e)` — a terminal I/O error occurred.
///
/// ## Notes
///
/// - Key release events (sent by crossterm on some platforms) are
///   silently skipped.
/// - `Ctrl-C` is mapped to cancellation at the runner layer so that
///   every component honours it without needing its own binding.
pub fn drive_event_loop<C, S, B, F>(
    terminal: &mut Terminal<B>,
    component: C,
    state: &mut S,
    mut read_event: F,
) -> io::Result<Option<S::Value>>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState,
    B: Backend,
    F: FnMut() -> io::Result<Event>,
{
    loop {
        terminal.draw(|f| {
            let area = f.area();
            let widget = component.clone();
            f.render_stateful_widget(widget, area, state);
        })?;

        match read_event()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                if is_ctrl_c(&key) {
                    return Ok(None);
                }
                match component.handle_event(state, key) {
                    EventOutcome::Submitted => return Ok(Some(state.value())),
                    EventOutcome::Cancelled => return Ok(None),
                    _ => {}
                }
            }
            Event::Resize(..) => {
                // terminal.draw on the next iteration picks up the
                // new size automatically; nothing else to do.
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
///   `Some(h)` runs inline using ratatui's [`Viewport::Inline`] for
///   `h` lines below the current cursor.
///
/// ## Returns
///
/// The submitted value from `state.value()`.
///
/// ## Errors
///
/// Returns [`io::ErrorKind::Interrupted`] on cancellation (Esc or
/// `Ctrl-C`). Propagates any other terminal I/O error.
pub fn run_standalone<C, S, V>(
    component: C,
    mut state: S,
    height: Option<u16>,
) -> io::Result<V>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState<Value = V>,
{
    let fullscreen = height.is_none();
    prepare_terminal(fullscreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let options = TerminalOptions {
        viewport: match height {
            Some(h) => Viewport::Inline(h),
            None => Viewport::Fullscreen,
        },
    };
    let mut terminal = Terminal::with_options(backend, options)?;

    let loop_result = drive_event_loop(&mut terminal, component, &mut state, event::read);

    restore_terminal(fullscreen);

    match loop_result? {
        Some(value) => Ok(value),
        None => Err(io::Error::new(CANCELLED_KIND, "cancelled")),
    }
}

fn prepare_terminal(fullscreen: bool) -> io::Result<()> {
    enable_raw_mode()?;
    if fullscreen {
        let mut out: Stdout = io::stdout();
        execute!(out, EnterAlternateScreen)?;
        out.flush().ok();
    }
    Ok(())
}

fn restore_terminal(fullscreen: bool) {
    if fullscreen {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
    let _ = disable_raw_mode();
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        backend::TestBackend,
        buffer::Buffer,
        layout::Rect,
        style::Style,
    };

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
    }

    impl StatefulWidget for Echo {
        type State = EchoState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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

    fn key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn run(events: Vec<Event>) -> io::Result<Option<String>> {
        let backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
        let mut state = EchoState::default();
        let mut iter = events.into_iter();
        drive_event_loop(&mut terminal, Echo, &mut state, || {
            iter.next()
                .ok_or_else(|| io::Error::other("no more events"))
        })
    }

    #[test]
    fn submits_on_enter() {
        let events = vec![
            key_event(KeyCode::Char('h')),
            key_event(KeyCode::Char('i')),
            key_event(KeyCode::Enter),
        ];
        let result = run(events).expect("drive loop");
        assert_eq!(result, Some("hi".to_string()));
    }

    #[test]
    fn cancels_on_esc() {
        let events = vec![key_event(KeyCode::Char('x')), key_event(KeyCode::Esc)];
        let result = run(events).expect("drive loop");
        assert_eq!(result, None);
    }

    #[test]
    fn cancels_on_ctrl_c() {
        let events = vec![Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ))];
        let result = run(events).expect("drive loop");
        assert_eq!(result, None);
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
        assert_eq!(result, Some("a".to_string()));
    }

    #[test]
    fn key_release_events_are_skipped() {
        let release = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        let events = vec![
            release,
            key_event(KeyCode::Char('b')),
            key_event(KeyCode::Enter),
        ];
        let result = run(events).expect("drive loop");
        assert_eq!(result, Some("b".to_string()));
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
}
