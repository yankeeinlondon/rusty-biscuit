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

use std::io::{self, IsTerminal, Stdout, Write};

use crossterm::{
    cursor::MoveTo,
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
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Clear, StatefulWidget},
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
                log_key_event_if_enabled(&key);
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
    let (effective_chrome, needs_overlay) = prepare_chrome_for_hint(chrome, help_hint);
    let overlay_hint = if needs_overlay { help_hint } else { None };
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|f| {
                render_frame(f, component.clone(), state, &effective_chrome, overlay_hint);
            })?;
            needs_redraw = false;
        }

        match read_event()? {
            Event::Key(key) => {
                log_key_event_if_enabled(&key);
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
///   `Some(HeightSpec::Cells(n))` runs inline for up to `n` rows below
///   the cursor (ratatui's autoresize clamps the viewport to the live
///   terminal height when the terminal is smaller than `n`).
///   `Some(HeightSpec::Percent(p))` resolves the percentage against the
///   current terminal height *and* re-resolves it on every resize event
///   so the inline viewport tracks the requested fraction of the
///   terminal as it grows or shrinks (clamped to a floor of 3 rows so
///   there is always room for a list plus an error row).
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
    // Redirect stdout to /dev/tty when the caller captured stdout
    // (e.g. `FOO=$(question ...)`). Restored on Drop, before the
    // CLI prints the result, so the caller's pipe receives only the
    // submitted value.
    let _stdout_redirect = StdoutTtyRedirect::activate_if_piped();
    let kbd_pushed = prepare_terminal(fullscreen)?;
    let mut guard = TerminalGuard::new(fullscreen, kbd_pushed);

    let hint_opt = if hint.is_empty() {
        None
    } else {
        Some(hint.as_str())
    };

    let loop_result: io::Result<LoopExit<V>> = match height {
        Some(spec @ HeightSpec::Percent(_)) => run_inline_dynamic_with_chrome(
            component,
            &mut state,
            spec,
            &chrome,
            hint_opt,
        ),
        Some(spec @ HeightSpec::Cells(_)) => {
            let resolved = resolve_height_spec(spec)?;
            let backend = CrosstermBackend::new(io::stdout());
            let options = TerminalOptions {
                viewport: Viewport::Inline(resolved),
            };
            let mut terminal = Terminal::with_options(backend, options)?;
            let result = drive_event_loop_with_chrome(
                &mut terminal,
                component,
                &mut state,
                event::read,
                hint_opt,
                &chrome,
            );
            // Whether the loop returned Ok(_) or Err(_), the inline
            // viewport must be finalized while the terminal is still
            // alive so we can hand the rendered rect back to the user
            // (or wipe it). Errors from finalization are swallowed —
            // the loop's outcome is the authoritative result.
            let _ = finalize_inline_viewport(&mut terminal, &chrome);
            result
        }
        None => {
            let backend = CrosstermBackend::new(io::stdout());
            let options = TerminalOptions {
                viewport: Viewport::Fullscreen,
            };
            let mut terminal = Terminal::with_options(backend, options)?;
            drive_event_loop_with_chrome(
                &mut terminal,
                component,
                &mut state,
                event::read,
                hint_opt,
                &chrome,
            )
        }
    };

    // Drain any pending input still in the terminal's buffer (e.g. a
    // cursor-position DSR response that arrived in the same poll as
    // the user's Enter keystroke). Done while raw mode is still on so
    // crossterm's parser silently discards non-public events like
    // CursorPosition; otherwise the bytes leak into the next program's
    // stdin (zsh would type "6;6R" at its prompt).
    drain_pending_events();
    guard.dismiss();
    restore_terminal(fullscreen, kbd_pushed);

    loop_exit_to_result(loop_result?)
}

/// Wraps up an inline viewport once its event loop has exited.
///
/// With `chrome.show_on_exit == false` (the default, fzf-style) the
/// viewport rows are cleared and the cursor is parked at the viewport's
/// top-left, so the next shell prompt overwrites the area the prompt
/// occupied. With `show_on_exit == true` the last-drawn frame is left
/// in place and the cursor is moved to the row immediately below the
/// chrome, so subsequent stdout output (the CLI result line, or the
/// next shell prompt) follows the rendered border without overlapping
/// it.
///
/// Only meaningful for [`Viewport::Inline`]; fullscreen prompts revert
/// to the original screen via [`LeaveAlternateScreen`] and don't go
/// through this path.
fn finalize_inline_viewport<B: Backend>(
    terminal: &mut Terminal<B>,
    chrome: &FrameChromeConfig,
) -> io::Result<()> {
    let area = terminal.get_frame().area();
    if chrome.show_on_exit {
        // Park cursor on a fresh row immediately below the rendered
        // chrome. The Inline viewport's `area.y` is its top-row offset
        // in absolute terminal coordinates, so `area.y + area.height`
        // is the first row past it.
        let target_y = area.y.saturating_add(area.height);
        execute!(io::stdout(), MoveTo(0, target_y))?;
    } else {
        // Replace each cell of the viewport with a space so the
        // rendered prompt disappears. Then position the cursor at the
        // viewport's top-left so the next shell prompt reuses the
        // space (matching fzf's behaviour).
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(Clear, area);
        })?;
        execute!(io::stdout(), MoveTo(area.x, area.y))?;
    }
    Ok(())
}

/// Consumes any pending events in crossterm's input buffer without
/// blocking.
///
/// Used at the tail of a prompt to discard orphan terminal responses
/// (notably DSR cursor-position replies) that would otherwise leak
/// into the next reader of `/dev/tty`. crossterm's parser handles
/// non-public events like [`crossterm::event::Event::CursorPosition`]
/// by reading and discarding them, so a `poll(0)` + `read` loop is
/// sufficient — we simply discard whatever it returns.
fn drain_pending_events() {
    while let Ok(true) = event::poll(std::time::Duration::ZERO) {
        if event::read().is_err() {
            break;
        }
    }
}

/// Recomputes the inline-viewport height for a percent spec on resize.
///
/// Returns `Some(new_h)` when the spec is [`HeightSpec::Percent`] and
/// resolving it against `new_term_rows` yields a value that differs
/// from `current_h`. Returns `None` for [`HeightSpec::Cells`] (ratatui's
/// own autoresize already clamps the viewport to the live terminal
/// height) and when the new value matches the current one.
///
/// Factored out so the dynamic-resize decision can be unit tested
/// without spinning up a real terminal.
fn maybe_recompute_inline_height(
    spec: HeightSpec,
    new_term_rows: u16,
    current_h: u16,
) -> Option<u16> {
    match spec {
        HeightSpec::Percent(_) => {
            let new_h = spec.resolve(new_term_rows);
            (new_h != current_h).then_some(new_h)
        }
        HeightSpec::Cells(_) => None,
    }
}

/// Inline event loop that recreates the [`Terminal`] when a resize
/// event changes the resolved viewport height.
///
/// Used only for [`HeightSpec::Percent`]: the percentage is re-resolved
/// against the new terminal row count on every [`Event::Resize`], and
/// when the result differs from the current inline height the loop
/// clears the old viewport, drops the [`Terminal`], and rebuilds it
/// with `Viewport::Inline(new_h)` at the cursor position. The component
/// state is preserved across recreations so the user keeps their
/// in-progress input.
///
/// For absolute [`HeightSpec::Cells`] this is unnecessary — ratatui's
/// own `autoresize` clamps the viewport to the live terminal height,
/// and the cap should not change with the terminal size.
fn run_inline_dynamic_with_chrome<C, S, V>(
    component: C,
    state: &mut S,
    spec: HeightSpec,
    chrome: &FrameChromeConfig,
    hint_opt: Option<&str>,
) -> io::Result<LoopExit<V>>
where
    C: Clone + StatefulWidget<State = S> + HandleEvent,
    S: StandaloneState<Value = V>,
{
    let mut current_h = resolve_height_spec(spec)?;

    'recreate: loop {
        let backend = CrosstermBackend::new(io::stdout());
        let options = TerminalOptions {
            viewport: Viewport::Inline(current_h),
        };
        let mut terminal = Terminal::with_options(backend, options)?;
        let mut needs_redraw = true;

        loop {
            if needs_redraw {
                draw_with_chrome(&mut terminal, component.clone(), state, hint_opt, chrome)?;
                needs_redraw = false;
            }

            match event::read()? {
                Event::Key(key) => {
                    log_key_event_if_enabled(&key);
                    if !should_dispatch_key_event(&key) {
                        continue;
                    }
                    if is_ctrl_c(&key) {
                        let _ = finalize_inline_viewport(&mut terminal, chrome);
                        return Ok(LoopExit::CtrlC);
                    }
                    match component.handle_event(state, key) {
                        EventOutcome::Submitted => {
                            let value = state.value();
                            let _ = finalize_inline_viewport(&mut terminal, chrome);
                            return Ok(LoopExit::Submitted(value));
                        }
                        EventOutcome::Cancelled => {
                            let _ = finalize_inline_viewport(&mut terminal, chrome);
                            return Ok(LoopExit::Esc);
                        }
                        EventOutcome::Consumed => {
                            needs_redraw = true;
                        }
                        EventOutcome::Ignored => {}
                    }
                }
                Event::Resize(_, new_rows) => {
                    if let Some(new_h) =
                        maybe_recompute_inline_height(spec, new_rows, current_h)
                    {
                        current_h = new_h;
                        let _ = terminal.clear();
                        drop(terminal);
                        continue 'recreate;
                    }
                    needs_redraw = true;
                }
                _ => {}
            }
        }
    }
}

/// Internal: runs a single redraw of `component` (optionally wrapped in
/// chrome) plus the bottom-of-area help hint, sharing the layout used
/// by [`drive_event_loop_with_chrome`] so the dynamic-resize loop above
/// renders identically.
fn draw_with_chrome<C, S, B>(
    terminal: &mut Terminal<B>,
    component: C,
    state: &mut S,
    help_hint: Option<&str>,
    chrome: &FrameChromeConfig,
) -> io::Result<()>
where
    C: StatefulWidget<State = S>,
    B: Backend,
{
    let (effective_chrome, needs_overlay) = prepare_chrome_for_hint(chrome, help_hint);
    let overlay_hint = if needs_overlay { help_hint } else { None };
    terminal.draw(|f| {
        render_frame(f, component, state, &effective_chrome, overlay_hint);
    })?;
    Ok(())
}

/// Renders one frame: the inner widget (optionally wrapped in chrome)
/// plus an overlay help hint, when present.
///
/// The hint is rendered inside the rect (skipping any left/right border
/// columns) so it never clobbers a vertical border glyph; when chrome
/// has a visible bottom border the caller is expected to have folded
/// the hint into [`FrameChromeConfig::bottom_label`] already (via
/// [`prepare_chrome_for_hint`]) and to pass `overlay_hint = None`.
fn render_frame<C, S>(
    f: &mut ratatui::Frame<'_>,
    component: C,
    state: &mut S,
    chrome: &FrameChromeConfig,
    overlay_hint: Option<&str>,
) where
    C: StatefulWidget<State = S>,
{
    let area = f.area();
    if chrome.is_empty() {
        f.render_stateful_widget(component, area, state);
    } else {
        let frame = FrameChrome::from_config(component, chrome);
        f.render_stateful_widget(frame, area, state);
    }
    if let Some(hint_text) = overlay_hint
        && !hint_text.is_empty()
        && area.height > 1
    {
        let (x, width) = hint_horizontal_extent(chrome, area);
        if width > 0 {
            let y = area.bottom().saturating_sub(1);
            let hint_line = Line::styled(
                hint_text.to_string(),
                Style::default().add_modifier(Modifier::DIM),
            );
            f.buffer_mut().set_line(x, y, &hint_line, width);
        }
    }
}

/// Folds the help hint into [`FrameChromeConfig::bottom_label`] when the
/// chrome has a visible bottom border.
///
/// Returns the chrome to render from and `true` when the caller still
/// needs to draw the hint as a free-floating overlay row (because the
/// chrome had no bottom border to host it).
///
/// Inlining the hint into the bottom border lets ratatui's [`Block`]
/// place it on the border line itself, leaving the rounded corner glyphs
/// at `area.x` / `area.right() - 1` intact. When the chrome has no
/// bottom border (e.g. [`BorderStyle::Top`]) or no border at all, the
/// caller still renders the hint as an overlay row — but the renderer
/// trims the row to skip vertical border columns so a `Vertical` chrome
/// keeps its left/right glyphs.
fn prepare_chrome_for_hint(
    chrome: &FrameChromeConfig,
    help_hint: Option<&str>,
) -> (FrameChromeConfig, bool) {
    let hint = help_hint.unwrap_or("");
    if hint.is_empty() {
        return (chrome.clone(), false);
    }
    if chrome.border.has_bottom() && chrome.bottom_label.is_none() {
        let mut adjusted = chrome.clone();
        adjusted.bottom_label = Some(hint.to_string());
        return (adjusted, false);
    }
    (chrome.clone(), true)
}

/// Computes the horizontal extent (`x`, `width`) for an overlay hint row
/// that must avoid vertical border glyphs.
///
/// Returns the start column and width to pass to
/// [`ratatui::buffer::Buffer::set_line`] so the hint never overwrites a
/// LEFT or RIGHT border glyph drawn by [`FrameChrome`].
fn hint_horizontal_extent(chrome: &FrameChromeConfig, area: Rect) -> (u16, u16) {
    let (sides, _) = chrome.border.ratatui_parts();
    let left_inset = if sides.contains(ratatui::widgets::Borders::LEFT) {
        1
    } else {
        0
    };
    let right_inset = if sides.contains(ratatui::widgets::Borders::RIGHT) {
        1
    } else {
        0
    };
    let x = area.x.saturating_add(left_inset);
    let width = area
        .width
        .saturating_sub(left_inset)
        .saturating_sub(right_inset);
    (x, width)
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
    // REPORT_ALL_KEYS_AS_ESCAPE_CODES is the kitty-protocol flag that
    // makes terminals emit press/release events for *bare* modifier
    // keys (Ctrl, Alt, Shift, Super) when no other key is involved.
    // Without it, most kitty-aware terminals (WezTerm in particular)
    // only emit modifier events as part of a chord — which means
    // holding bare Ctrl never produces a key event and the
    // hotkey-badge UX silently does nothing. REPORT_EVENT_TYPES is
    // still required so press/release are distinguishable;
    // DISAMBIGUATE_ESCAPE_CODES keeps Esc from looking like a CSI
    // prefix.
    let kbd_pushed = execute!(
        out,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
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

/// Redirects fd 1 (`stdout`) to `/dev/tty` for the duration of a
/// standalone prompt when the process's real stdout is not a TTY
/// (e.g. when running under `FOO=$(question ...)` command
/// substitution).
///
/// This is required because `crossterm::cursor::position` — which
/// `ratatui` calls implicitly when constructing an inline-viewport
/// terminal — writes the DSR (`ESC[6n`) cursor-position query to
/// `io::stdout()` directly. If stdout is a pipe, the query never
/// reaches the terminal and crossterm times out with "The cursor
/// position could not be read within a normal duration".
///
/// On `Drop` the original stdout file descriptor is restored, so the
/// caller's result-emitting `println!` / `writeln!` calls reach the
/// captured pipe as expected.
///
/// On non-Unix platforms this is a no-op.
#[cfg(unix)]
struct StdoutTtyRedirect {
    saved_fd: Option<libc::c_int>,
    tty_fd: Option<libc::c_int>,
}

#[cfg(unix)]
impl StdoutTtyRedirect {
    /// Activates the redirect when stdout is not a TTY.
    ///
    /// Returns an inactive guard (no-op on drop) when stdout is
    /// already a TTY, when `/dev/tty` cannot be opened, or when any
    /// of the `dup`/`dup2` calls fail. In those failure modes the
    /// caller still proceeds — they will see the underlying error
    /// (e.g. crossterm's cursor-position timeout) rather than a
    /// confusing redirect-related failure.
    fn activate_if_piped() -> Self {
        if io::stdout().is_terminal() {
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        // SAFETY: passing a static, NUL-terminated C string and
        // standard libc flags. The returned fd is checked for error.
        let tty_fd = unsafe {
            libc::open(
                c"/dev/tty".as_ptr(),
                libc::O_WRONLY | libc::O_CLOEXEC,
            )
        };
        if tty_fd < 0 {
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        // SAFETY: STDOUT_FILENO is a valid open fd at process start.
        let saved_fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
        if saved_fd < 0 {
            unsafe { libc::close(tty_fd) };
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        // SAFETY: tty_fd is open; STDOUT_FILENO is valid.
        if unsafe { libc::dup2(tty_fd, libc::STDOUT_FILENO) } < 0 {
            unsafe {
                libc::close(saved_fd);
                libc::close(tty_fd);
            }
            return Self {
                saved_fd: None,
                tty_fd: None,
            };
        }
        Self {
            saved_fd: Some(saved_fd),
            tty_fd: Some(tty_fd),
        }
    }
}

#[cfg(unix)]
impl Drop for StdoutTtyRedirect {
    fn drop(&mut self) {
        // Flush any TUI bytes still buffered in Rust's stdout handle
        // BEFORE the fd swap, so they land on /dev/tty rather than
        // leaking into the caller's captured pipe.
        let _ = io::stdout().flush();
        if let Some(saved) = self.saved_fd.take() {
            // SAFETY: saved is an fd we created via dup; STDOUT_FILENO
            // is valid.
            unsafe {
                libc::dup2(saved, libc::STDOUT_FILENO);
                libc::close(saved);
            }
        }
        if let Some(tty) = self.tty_fd.take() {
            unsafe { libc::close(tty) };
        }
    }
}

#[cfg(not(unix))]
struct StdoutTtyRedirect;

#[cfg(not(unix))]
impl StdoutTtyRedirect {
    fn activate_if_piped() -> Self {
        Self
    }
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn should_dispatch_key_event(key: &KeyEvent) -> bool {
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => true,
        KeyEventKind::Release => {
            // Bare-modifier releases drive bare-Ctrl/Alt badge clearing.
            if matches!(key.code, KeyCode::Modifier(_)) {
                return true;
            }
            // `Ctrl+Space` and `Alt+Space` release events drive the
            // press-and-hold sticky-toggle UX in the choose
            // components. We deliberately ONLY pass through these
            // specific chord shapes — otherwise releases of arbitrary
            // chords (e.g. Esc, Enter) would reach component handlers
            // that don't expect them and could mis-fire bindings.
            //
            // Both space encodings are accepted: standard
            // `Char(' ') + CONTROL/ALT` (kitty protocol) and legacy
            // `Char('\0') + CONTROL` (Ctrl subtracts 0x40 from
            // Space's 0x20 → NUL).
            let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let has_alt = key.modifiers.contains(KeyModifiers::ALT);
            let is_space = matches!(key.code, KeyCode::Char(' '))
                || (matches!(key.code, KeyCode::Char('\0')) && has_ctrl);
            is_space && (has_ctrl ^ has_alt)
        }
    }
}

/// Optional diagnostic: when `BISCUIT_TUI_TRACE_KEYS=1` is set in the
/// environment, dump every key event the runner observes to a log
/// file in the system temp dir (`$TMPDIR/biscuit-tui-keys.log`).
///
/// Useful for diagnosing "press X but nothing happens" complaints —
/// distinguishes the case where the binary never receives an event
/// (terminal config / encoder issue) from the case where it does
/// receive one but doesn't act on it (binary handler bug).
///
/// Logging goes to a file rather than stderr because in TUI mode
/// stderr is wired into the alternate-screen buffer and would
/// scramble the prompt. Tail the log with `tail -f
/// /tmp/biscuit-tui-keys.log` while exercising the binary.
///
/// Cost when disabled: a single env-var lookup per key event.
fn log_key_event_if_enabled(key: &KeyEvent) {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    let enabled =
        *ENABLED.get_or_init(|| std::env::var("BISCUIT_TUI_TRACE_KEYS").as_deref() == Ok("1"));
    if !enabled {
        return;
    }
    let path = std::env::temp_dir().join("biscuit-tui-keys.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write as _;
        let _ = writeln!(
            f,
            "{:?}  code={:?}  modifiers={:?}  kind={:?}  state={:?}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            key.code,
            key.modifiers,
            key.kind,
            key.state,
        );
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
        assert_ne!(
            buf[(29, 4)].symbol(),
            " ",
            "bottom-right corner clobbered",
        );
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
}
