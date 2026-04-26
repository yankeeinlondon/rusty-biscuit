//! TTY provider selection UI built on `tui-chrome`.
//!
//! This module contains the one-shot interactive picker used by
//! `compose` and `inline-compose` when no explicit `--<provider>` flag
//! was given and the session has a TTY.

use std::io;

use claudine::composition::{ProviderPickerOption, ProviderPickerPlan};
use claudine::events::Provider;
use tui_chrome::prelude::*;

/// Prompt the user to select a single provider from a picker plan.
///
/// Built on [`tui_chrome::ChooseOne`] + [`tui_chrome::run_standalone`].
/// Maps each [`ProviderPickerOption`] to a [`ChoiceOption`], honours
/// `plan.default_index` as the initial selection, and translates
/// [`EventOutcome::Submitted`] into the selected [`Provider`] and
/// [`EventOutcome::Cancelled`] into an abort error.
///
/// ## Errors
///
/// Returns an `io::Error` with kind [`ABORTED_KIND`] when the user
/// presses `Esc`, and [`CANCELLED_KIND`] on `Ctrl-C`.
pub fn prompt_one_shot_provider(plan: ProviderPickerPlan) -> io::Result<Provider> {
    let options: Vec<ChoiceOption<Provider>> = plan
        .options
        .iter()
        .map(provider_option_to_choice)
        .collect();

    let mut state = ChooseOneState::from_options(options);

    // Pre-select the default index by its stable id (provider slug).
    if let Some(default_opt) = plan.options.get(plan.default_index) {
        state = state.with_initial_selection(default_opt.provider.as_slug());
    }

    let selected: Option<Provider> = run_standalone(ChooseOne::new(), state, None)?;

    selected.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no provider selected"))
}

fn provider_option_to_choice(opt: &ProviderPickerOption) -> ChoiceOption<Provider> {
    let label = format!("{}", opt.provider);
    let id = opt.provider.as_slug().to_string();
    let choice = ChoiceOption::new(id, label, opt.provider);
    // If this option has a rank reason, we could decorate the label,
    // but for now keep labels clean.
    choice
}


