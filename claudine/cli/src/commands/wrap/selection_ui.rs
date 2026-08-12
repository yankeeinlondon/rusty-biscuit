//! TTY provider selection UI built on `biscuit-tui`.
//!
//! This module contains the one-shot interactive picker used by
//! `compose` and `inline-compose` when no explicit `--<provider>` flag
//! was given and the session has a TTY, plus the sequence review
//! screen built on `biscuit-tui::InputTable`.

use std::io;

use biscuit_tui::components::input_table::TextInputConfig;
use biscuit_tui::prelude::*;
use claudine::composition::{
    ProviderPickerOption, ProviderPickerPlan, ResolvedExecutionTarget, SequenceStepDraft,
};
use claudine::provider::Provider;

/// Prompt the user to select a single provider from a picker plan.
///
/// Built on [`biscuit_tui::ChooseOne`] + [`biscuit_tui::run_standalone`].
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
    let options: Vec<ChoiceOption<Provider>> =
        plan.options.iter().map(provider_option_to_choice).collect();

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
/// Built on [`biscuit_tui::InputTable`] + [`biscuit_tui::run_standalone`].
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
    catalog: &claudine::model_catalog::ModelCatalogService,
) -> io::Result<Vec<ResolvedExecutionTarget>> {
    if drafts.is_empty() {
        return Ok(Vec::new());
    }

    let columns = build_columns(&drafts, catalog);
    let initial_rows = build_initial_rows(&drafts, catalog);
    let state = InputTableState::new(columns, initial_rows);

    let rows: Vec<Row> = run_standalone(InputTable::new(), state, None)?;

    // Decode each row back into a ResolvedExecutionTarget.
    let mut targets = Vec::with_capacity(rows.len());
    for row in &rows {
        targets.push(decode_execution_target(row)?);
    }

    Ok(targets)
}

/// Determine whether all drafts share a single unified provider.
///
/// Returns `Some(provider)` when every row resolves to the same
/// provider (either because it is locked or because every draft's
/// default plan points to the same provider).  This lets the model
/// column use a `ChooseOne` backed by that provider's catalog.
fn determine_unified_provider(drafts: &[SequenceStepDraft]) -> Option<Provider> {
    if drafts.is_empty() {
        return None;
    }

    // If any draft is provider-locked, all share the same resolved provider.
    if let Some(locked) = drafts
        .iter()
        .find_map(|d| d.provider_locked.then_some(d.resolved_provider).flatten())
    {
        return Some(locked);
    }

    // Check if all drafts have the same default provider in their plan.
    let first_default = drafts
        .first()?
        .provider_plan
        .options
        .get(drafts.first()?.provider_plan.default_index)?
        .provider;
    let all_same = drafts.iter().all(|d| {
        d.provider_plan
            .options
            .get(d.provider_plan.default_index)
            .map(|o| o.provider)
            == Some(first_default)
    });

    if all_same { Some(first_default) } else { None }
}

/// Build the shared column schema for the sequence review table.
fn build_columns(
    drafts: &[SequenceStepDraft],
    catalog: &claudine::model_catalog::ModelCatalogService,
) -> Vec<InputTableColumn> {
    let provider_locked = drafts.first().map(|d| d.provider_locked).unwrap_or(false);
    let model_locked = drafts.first().map(|d| d.model_locked).unwrap_or(false);

    // Provider column
    let provider_column = if provider_locked {
        InputTableColumn::StaticText {
            id: "provider".into(),
            text: "Provider".into(),
        }
    } else {
        InputTableColumn::ChooseOne(
            ChoiceInput::new("provider", "Provider").with_options(
                drafts[0]
                    .provider_plan
                    .options
                    .iter()
                    .map(|o| {
                        ChoiceOption::new(
                            o.provider.as_slug(),
                            format!("{}", o.provider),
                            o.provider.as_slug(),
                        )
                    })
                    .collect(),
            ),
        )
    };

    // Model column
    let unified_provider = determine_unified_provider(drafts);
    let model_catalog = unified_provider
        .map(|p| catalog.catalog_for(p))
        .unwrap_or_default();

    let model_column = if model_locked {
        InputTableColumn::StaticText {
            id: "model".into(),
            text: "Model".into(),
        }
    } else if !model_catalog.is_empty() {
        let mut options = vec![ChoiceOption::new("__default__", "(default)", "__default__")];
        for model in &model_catalog {
            options.push(ChoiceOption::new(
                model.clone(),
                model.clone(),
                model.clone(),
            ));
        }
        InputTableColumn::ChooseOne(ChoiceInput::new("model", "Model").with_options(options))
    } else {
        InputTableColumn::TextInput {
            id: "model".into(),
            config: TextInputConfig::default(),
        }
    };

    vec![
        InputTableColumn::StaticText {
            id: "step".into(),
            text: "Step".into(),
        },
        provider_column,
        model_column,
    ]
}

/// Build the initial row values from the sequence step drafts.
fn build_initial_rows(
    drafts: &[SequenceStepDraft],
    catalog: &claudine::model_catalog::ModelCatalogService,
) -> Vec<Row> {
    let provider_locked = drafts.first().map(|d| d.provider_locked).unwrap_or(false);
    let model_locked = drafts.first().map(|d| d.model_locked).unwrap_or(false);
    let unified_provider = determine_unified_provider(drafts);
    let model_catalog = unified_provider
        .map(|p| catalog.catalog_for(p))
        .unwrap_or_default();

    drafts
        .iter()
        .map(|draft| {
            let provider_value = if provider_locked {
                CellValue::StaticText(
                    draft
                        .resolved_provider
                        .map(|p| format!("{}", p))
                        .unwrap_or_default(),
                )
            } else {
                CellValue::ChosenOne(
                    draft
                        .provider_plan
                        .options
                        .get(draft.provider_plan.default_index)
                        .map(|o| o.provider.as_slug().to_string()),
                )
            };

            let model_value = if model_locked {
                CellValue::StaticText(
                    draft
                        .proposed_model
                        .clone()
                        .unwrap_or_else(|| "(default)".into()),
                )
            } else if !model_catalog.is_empty() {
                let model_id = draft
                    .proposed_model
                    .as_deref()
                    .and_then(|m| {
                        if model_catalog.iter().any(|cat| cat.eq_ignore_ascii_case(m)) {
                            Some(m.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "__default__".to_string());
                CellValue::ChosenOne(Some(model_id))
            } else {
                CellValue::Text(draft.proposed_model.clone().unwrap_or_default())
            };

            Row::new(vec![
                RowCell::new(
                    "step",
                    CellValue::StaticText(format!("{} {}", draft.step_index + 1, draft.step_name)),
                ),
                RowCell::new("provider", provider_value),
                RowCell::new("model", model_value),
            ])
        })
        .collect()
}

/// Decode a single [`Row`] into a [`ResolvedExecutionTarget`].
///
/// ## Errors
///
/// Returns an `io::Error` with kind [`InvalidData`](io::ErrorKind::InvalidData)
/// when the provider cell is missing, has an unexpected type, or contains an
/// unknown provider slug.
fn decode_execution_target(row: &Row) -> io::Result<ResolvedExecutionTarget> {
    let provider = match row.get("provider") {
        Some(CellValue::ChosenOne(Some(slug))) => {
            Provider::parse_cli_name(slug).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown provider slug in review: {slug}"),
                )
            })?
        }
        Some(CellValue::StaticText(text)) => Provider::parse_cli_name(text).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown provider in locked column: {text}"),
            )
        })?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "provider cell missing or has unexpected value type",
            ));
        }
    };

    let model = match row.get("model") {
        Some(CellValue::Text(s)) | Some(CellValue::StaticText(s)) => {
            if s.is_empty() || s == "(default)" {
                None
            } else {
                Some(s.clone())
            }
        }
        Some(CellValue::ChosenOne(Some(id))) => {
            if id == "__default__" {
                None
            } else {
                Some(id.clone())
            }
        }
        _ => None,
    };

    Ok(ResolvedExecutionTarget {
        provider,
        provider_reason: claudine::composition::ProviderResolutionReason::SequenceReview,
        model,
        model_reason: claudine::composition::ModelResolutionReason::SequenceReview,
    })
}

fn provider_option_to_choice(opt: &ProviderPickerOption) -> ChoiceOption<Provider> {
    let label = format!("{}", opt.provider);
    let id = opt.provider.as_slug().to_string();
    let choice = ChoiceOption::new(id, label, opt.provider);
    // If this option has a rank reason, we could decorate the label,
    // but for now keep labels clean.
    choice
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::ModelResolutionReason;
    use claudine::config::claudine_config::{
        DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
    };
    use claudine::model_catalog::ModelCatalogService;
    use std::collections::HashMap;

    fn make_draft(
        provider: Provider,
        provider_locked: bool,
        model_locked: bool,
        model: Option<String>,
    ) -> SequenceStepDraft {
        SequenceStepDraft {
            step_index: 0,
            step_name: "Test".into(),
            provider_plan: ProviderPickerPlan {
                options: vec![ProviderPickerOption {
                    provider,
                    rank_reason: None,
                }],
                default_index: 0,
            },
            proposed_model: model,
            model_reason: ModelResolutionReason::ProviderDefault,
            provider_locked,
            model_locked,
            resolved_provider: if provider_locked {
                Some(provider)
            } else {
                None
            },
        }
    }

    fn make_row(provider_value: CellValue, model_value: CellValue) -> Row {
        Row::new(vec![
            RowCell::new("step", CellValue::StaticText("1 Test".into())),
            RowCell::new("provider", provider_value),
            RowCell::new("model", model_value),
        ])
    }

    #[test]
    fn locked_provider_column_uses_static_text() {
        let draft = make_draft(Provider::Claude, true, false, None);
        let catalog = ModelCatalogService::new();
        let columns = build_columns(&[draft], &catalog);
        let provider_col = columns.iter().find(|c| c.id() == "provider").unwrap();
        assert!(matches!(provider_col, InputTableColumn::StaticText { .. }));
    }

    #[test]
    fn locked_model_column_uses_static_text() {
        let draft = make_draft(Provider::Claude, false, true, Some("gpt-4".into()));
        let catalog = ModelCatalogService::new();
        let columns = build_columns(&[draft], &catalog);
        let model_col = columns.iter().find(|c| c.id() == "model").unwrap();
        assert!(matches!(model_col, InputTableColumn::StaticText { .. }));
    }

    #[test]
    fn unlocked_provider_column_uses_choose_one() {
        let draft = make_draft(Provider::Claude, false, false, None);
        let catalog = ModelCatalogService::new();
        let columns = build_columns(&[draft], &catalog);
        let provider_col = columns.iter().find(|c| c.id() == "provider").unwrap();
        assert!(matches!(provider_col, InputTableColumn::ChooseOne(_)));
    }

    #[test]
    fn model_column_choose_one_when_catalog_available() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Codex,
            ProviderModelOverride::AddList(vec!["gpt-5".into()]),
        );
        let catalog = ModelCatalogService::with_overrides(overrides);
        let draft = make_draft(Provider::Codex, false, false, None);
        let columns = build_columns(&[draft], &catalog);
        let model_col = columns.iter().find(|c| c.id() == "model").unwrap();
        assert!(matches!(model_col, InputTableColumn::ChooseOne(_)));
    }

    #[test]
    fn model_column_text_input_when_catalog_empty() {
        // Every provider now ships a research-fed static catalog
        // (generator v1), so an empty catalog must be constructed
        // explicitly via a replace-mode override with no values.
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Gemini,
            ProviderModelOverride::Detailed(DetailedModelOverride {
                mode: ModelOverrideMode::Replace,
                values: vec![],
            }),
        );
        let catalog = ModelCatalogService::with_overrides(overrides);
        let draft = make_draft(Provider::Gemini, false, false, None);
        let columns = build_columns(&[draft], &catalog);
        let model_col = columns.iter().find(|c| c.id() == "model").unwrap();
        assert!(matches!(model_col, InputTableColumn::TextInput { .. }));
    }

    #[test]
    fn decode_chosen_one_provider_exact_match() {
        let row = make_row(
            CellValue::ChosenOne(Some("codex".into())),
            CellValue::Text("".into()),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.provider, Provider::Codex);
    }

    #[test]
    fn decode_static_text_provider_exact_match() {
        let row = make_row(
            CellValue::StaticText("claude".into()),
            CellValue::Text("".into()),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.provider, Provider::Claude);
    }

    #[test]
    fn decode_unknown_provider_slug_errors() {
        let row = make_row(
            CellValue::ChosenOne(Some("unknown".into())),
            CellValue::Text("".into()),
        );
        let result = decode_execution_target(&row);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("unknown provider slug"));
    }

    #[test]
    fn decode_missing_provider_errors() {
        let row = Row::new(vec![
            RowCell::new("step", CellValue::StaticText("1 Test".into())),
            RowCell::new("model", CellValue::Text("".into())),
        ]);
        let result = decode_execution_target(&row);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("provider cell missing"));
    }

    #[test]
    fn decode_model_text_empty_becomes_none() {
        let row = make_row(
            CellValue::ChosenOne(Some("codex".into())),
            CellValue::Text("".into()),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.model, None);
    }

    #[test]
    fn decode_model_text_non_empty_preserved() {
        let row = make_row(
            CellValue::ChosenOne(Some("codex".into())),
            CellValue::Text("gpt-5".into()),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.model, Some("gpt-5".into()));
    }

    #[test]
    fn decode_model_choose_one_default_option() {
        let row = make_row(
            CellValue::ChosenOne(Some("codex".into())),
            CellValue::ChosenOne(Some("__default__".into())),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.model, None);
    }

    #[test]
    fn decode_model_choose_one_specific_model() {
        let row = make_row(
            CellValue::ChosenOne(Some("codex".into())),
            CellValue::ChosenOne(Some("gpt-5".into())),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.model, Some("gpt-5".into()));
    }

    #[test]
    fn decode_model_text_input_empty() {
        let row = make_row(
            CellValue::ChosenOne(Some("codex".into())),
            CellValue::Text("".into()),
        );
        let target = decode_execution_target(&row).unwrap();
        assert_eq!(target.model, None);
    }

    #[test]
    fn multiple_drafts_maintain_row_order() {
        let draft1 = make_draft(Provider::Claude, false, false, None);
        let draft2 = SequenceStepDraft {
            step_index: 1,
            step_name: "Step 2".into(),
            ..make_draft(Provider::Codex, false, false, None)
        };
        let catalog = ModelCatalogService::new();
        let rows = build_initial_rows(&[draft1, draft2], &catalog);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("step"),
            Some(&CellValue::StaticText("1 Test".into()))
        );
        assert_eq!(
            rows[1].get("step"),
            Some(&CellValue::StaticText("2 Step 2".into()))
        );
    }
}
