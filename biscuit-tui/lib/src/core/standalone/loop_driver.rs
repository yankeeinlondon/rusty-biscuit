use super::*;

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
    // ratatui 0.30 gave `Backend` an associated `Error` type (already
    // bounded by `core::error::Error`); `Send + Sync + 'static` lets
    // `io::Error::other` wrap a draw failure from any backend.
    B::Error: Send + Sync + 'static,
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
    // ratatui 0.30 gave `Backend` an associated `Error` type (already
    // bounded by `core::error::Error`); `Send + Sync + 'static` lets
    // `io::Error::other` wrap a draw failure from any backend.
    B::Error: Send + Sync + 'static,
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
            })
            .map_err(io::Error::other)?;
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
    // ratatui 0.30 gave `Backend` an associated `Error` type (already
    // bounded by `core::error::Error`); `Send + Sync + 'static` lets
    // `io::Error::other` wrap a draw failure from any backend.
    B::Error: Send + Sync + 'static,
    F: FnMut() -> io::Result<Event>,
{
    let (effective_chrome, needs_overlay) = prepare_chrome_for_hint(chrome, help_hint);
    let overlay_hint = if needs_overlay { help_hint } else { None };
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|f| {
                render_frame(f, component.clone(), state, &effective_chrome, overlay_hint);
            })
            .map_err(io::Error::other)?;
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
