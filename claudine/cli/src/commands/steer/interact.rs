//! The human decisions `claudine steer` asks for: which session to steer,
//! and whether to interrupt it.
//!
//! [`Interaction`] is the seam; [`TerminalInteraction`] drives biscuit-tui
//! prompts, and tests script the answers. A prompt that cannot run, or is
//! dismissed, is never read as consent.

use std::io;

use biscuit_tui::prelude::*;
use claudine::steering::discovery::SessionListing;
use claudine::steering::identity::SteeringTargetId;
use ratatui::style::Modifier;

use super::render;

/// The human decisions the steer command needs.
pub(crate) trait Interaction {
    /// Lets the user pick one selectable row. `None` means the user cancelled.
    fn choose(&mut self, rows: &[SessionListing]) -> io::Result<Option<SteeringTargetId>>;

    /// Asks whether to interrupt the session `row` describes and then send.
    /// The caller has already shown what interruption would stop; `false`
    /// means declined or dismissed.
    fn confirm_interruption(&mut self, row: &SessionListing) -> io::Result<bool>;
}

/// biscuit-tui prompts on the controlling terminal.
pub(crate) struct TerminalInteraction;

/// The picker's rows as choice options. Every row is listed; an unavailable
/// row is a disabled option, so it is visible but cannot be selected.
pub(crate) fn picker_state(rows: &[SessionListing]) -> ChooseOneState<String> {
    let options = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let id = row.id.to_string();
            let option = ChoiceOption::new(id.clone(), render::picker_label(index, row), id);
            if row.is_selectable() { option } else { option.disabled() }
        })
        .collect();
    let input = ChoiceInput::new("steer-target", "Select the session to steer").with_options(options).required();
    ChooseOneState::new(input)
        .with_label(Label::new("Select the session to steer", LabelPosition::Above))
        .with_theme(picker_theme())
}

/// The default theme, with unavailable options also struck through: dim
/// alone is easy to miss, and the struck row still reads as text.
pub(crate) fn picker_theme() -> ComponentTheme {
    let mut theme = ComponentTheme::default();
    theme.disabled_style = theme.disabled_style.add_modifier(Modifier::DIM | Modifier::CROSSED_OUT);
    theme
}

/// Label, options, validation error, and help rows, capped so a long list
/// scrolls instead of taking the screen.
fn picker_height(options: usize) -> Option<HeightSpec> {
    let rows = options.saturating_add(3).min(14);
    Some(HeightSpec::Cells(u16::try_from(rows).unwrap_or(14)))
}

/// Maps a dismissed prompt to `None`; other terminal errors propagate.
fn dismissed<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == CANCELLED_KIND || error.kind() == ABORTED_KIND => Ok(None),
        Err(error) => Err(error),
    }
}

impl Interaction for TerminalInteraction {
    fn choose(&mut self, rows: &[SessionListing]) -> io::Result<Option<SteeringTargetId>> {
        let state = picker_state(rows);
        let picked: Option<Option<String>> =
            dismissed(tokio::task::block_in_place(|| run_standalone(ChooseOne::new(), state, picker_height(rows.len()))))?;
        Ok(picked.flatten().and_then(|id| rows.iter().find(|row| row.id.to_string() == id)).map(|row| row.id.clone()))
    }

    fn confirm_interruption(&mut self, _row: &SessionListing) -> io::Result<bool> {
        let state = BooleanSwitchState::new()
            .with_value(false)
            .with_label(Label::new("Interrupt the running turn, then send the message?", LabelPosition::Above));
        let answer = dismissed(tokio::task::block_in_place(|| run_standalone(BooleanSwitch::new(), state, Some(HeightSpec::Cells(2)))))?;
        Ok(answer == Some(true))
    }
}
