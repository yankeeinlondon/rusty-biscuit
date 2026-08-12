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

mod inline_viewport;
mod loop_driver;
mod terminal_lifecycle;

pub use loop_driver::{drive_event_loop, drive_event_loop_with_chrome, drive_event_loop_with_hint};

use inline_viewport::{finalize_inline_viewport, run_inline_dynamic_with_chrome};
use terminal_lifecycle::{
    TerminalGuard, drain_pending_events, prepare_terminal, restore_terminal,
};
// F2: StdoutTtyRedirect has a real impl on both Unix (`/dev/tty`) and
// Windows (`CONOUT$`); only exotic non-Unix/non-Windows targets fall back
// to the local no-op guard below.
#[cfg(any(unix, windows))]
use terminal_lifecycle::StdoutTtyRedirect;

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
    // Bail before touching `/dev/tty` when both stdio streams are
    // captured (the test-harness / pipeline shape). `enable_raw_mode`
    // disables `OPOST` on the shared controlling terminal, so a
    // subprocess that enters raw mode and exits leaves the *parent's*
    // tty with `\n` mapped to bare LF — corrupting any concurrent
    // redrawing UI in the parent (e.g. nextest's progress bar). The
    // shell command-substitution case (`FOO=$(question ...)`) still
    // works because stderr stays attached to the terminal there.
    if !io::stdout().is_terminal() && !io::stderr().is_terminal() {
        return Err(io::Error::other(
            "no interactive terminal available (stdout and stderr are both not a tty)",
        ));
    }
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
        Some(spec @ HeightSpec::Percent(_)) => {
            run_inline_dynamic_with_chrome(component, &mut state, spec, &chrome, hint_opt)
        }
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

// Exotic non-Unix / non-Windows targets (e.g. wasm, redox) have no
// equivalent of /dev/tty or CONOUT$. The redirect stays a no-op there;
// mod.rs still bails with "no interactive terminal available" when both
// stdio streams are captured. Unix and Windows pull in the real impl from
// terminal_lifecycle via the `#[cfg(any(unix, windows))]` import above.
#[cfg(all(not(unix), not(windows)))]
struct StdoutTtyRedirect;

#[cfg(all(not(unix), not(windows)))]
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
mod tests;
