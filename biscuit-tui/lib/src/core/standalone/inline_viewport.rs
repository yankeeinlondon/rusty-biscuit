use super::*;

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
pub(super) fn finalize_inline_viewport<B: Backend>(
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
pub(super) fn maybe_recompute_inline_height(
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
pub(super) fn run_inline_dynamic_with_chrome<C, S, V>(
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
                    if let Some(new_h) = maybe_recompute_inline_height(spec, new_rows, current_h) {
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
