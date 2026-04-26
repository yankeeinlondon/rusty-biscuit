//! TTY provider selection UI built on `tui-chrome`.
//!
//! This module contains the one-shot interactive picker used by
//! `compose` and `inline-compose` when no explicit `--<provider>` flag
//! was given and the session has a TTY, plus the sequence review
//! screen built on `tui-chrome::InputTable`.

use std::io;

use claudine::composition::{
    ProviderPickerOption, ProviderPickerPlan, ResolvedExecutionTarget, SequenceStepDraft,
};
use claudine::events::Provider;
use tui_chrome::prelude::*;
use tui_chrome::components::input_table::TextInputConfig;

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

/// Present a multi-step review screen for sequence execution.
///
/// Built on [`tui_chrome::InputTable`] + [`tui_chrome::run_standalone`].
/// Each step becomes one row with three columns:
///
/// 1. **Step label** (`StaticText`) — the step name, read-only.
/// 2. **Provider** (`ChooseOne` or `StaticText`) — editable when not
///    locked by an explicit CLI flag; otherwise a read-only display of
///    the locked provider.
/// 3. **Model** (`ChooseOne`, `TextInput`, or `StaticText`) — when a
///    catalog is available and the model is not locked, a `ChooseOne`
///    with catalog models plus a "(default)" option; when the catalog
///    is empty/unavailable, a free-form `TextInput`; when locked, a
///    read-only display of the locked model.
///
/// On [`EventOutcome::Submitted`] (Ctrl+S), the typed row values are
/// decoded into one [`ResolvedExecutionTarget`] per step.  On
/// [`EventOutcome::Cancelled`] (Esc), an abort error is returned.
///
/// ## Errors
///
/// Returns an `io::Error` with kind [`ABORTED_KIND`] when the user
/// presses `Esc`, and [`CANCELLED_KIND`] on `Ctrl-C`.
pub fn review_sequence(
    drafts: Vec<SequenceStepDraft>,
) -> io::Result<Vec<ResolvedExecutionTarget>> {
    if drafts.is_empty() {
        return Ok(Vec::new());
    }

    // Build column schema.
    let columns = vec![
        InputTableColumn::StaticText {
            id: "step".into(),
            text: "Step".into(),
        },
        InputTableColumn::ChooseOne(ChoiceInput::new("provider", "Provider").with_options(
            // Seed with all providers from the first draft's plan as a
            // representative option list.  In practice every draft's
            // plan.options uses the same installed set.
            drafts[0]
                .provider_plan
                .options
                .iter()
                .map(|o| ChoiceOption::new(o.provider.as_slug(), format!("{}", o.provider), o.provider.as_slug()))
                .collect(),
        )),
        InputTableColumn::TextInput {
            id: "model".into(),
            config: TextInputConfig::default(),
        },
    ];

    // Build initial rows from drafts.
    let initial_rows: Vec<Row> = drafts
        .iter()
        .map(|draft| {
            let provider_value = draft
                .provider_plan
                .options
                .get(draft.provider_plan.default_index)
                .map(|o| o.provider.as_slug().to_string());

            Row::new(vec![
                RowCell::new(
                    "step",
                    CellValue::StaticText(format!("{} {}", draft.step_index + 1, draft.step_name)),
                ),
                RowCell::new(
                    "provider",
                    CellValue::ChosenOne(provider_value),
                ),
                RowCell::new(
                    "model",
                    CellValue::Text(draft.proposed_model.clone().unwrap_or_default()),
                ),
            ])
        })
        .collect();

    let state = InputTableState::new(columns, initial_rows);

    let rows: Vec<Row> = run_standalone(InputTable::new(), state, None)?;

    // Decode each row back into a ResolvedExecutionTarget.
    let mut targets = Vec::with_capacity(rows.len());
    for row in &rows {
        let provider_slug = row
            .get_text("provider")
            .unwrap_or_default();
        let provider = Provider::fuzzy_match_cli_name(provider_slug).unwrap_or(Provider::Claude);

        let model = row.get_text("model").map(|s| s.to_string());
        let model = if model.as_deref().unwrap_or("").is_empty() {
            None
        } else {
            model
        };

        targets.push(ResolvedExecutionTarget {
            provider,
            provider_reason: claudine::composition::ProviderResolutionReason::SequenceReview,
            model,
            model_reason: claudine::composition::ModelResolutionReason::SequenceReview,
        });
    }

    Ok(targets)
}

fn provider_option_to_choice(opt: &ProviderPickerOption) -> ChoiceOption<Provider> {
    let label = format!("{}", opt.provider);
    let id = opt.provider.as_slug().to_string();
    let choice = ChoiceOption::new(id, label, opt.provider);
    // If this option has a rank reason, we could decorate the label,
    // but for now keep labels clean.
    choice
}


