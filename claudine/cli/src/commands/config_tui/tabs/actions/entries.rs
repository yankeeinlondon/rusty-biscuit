use std::collections::HashMap;

use claudine::actions::HookAction;
use claudine::events::AgenticEvent;

use super::super::super::app::{ActionView, App};
use super::{ActionEntry, ActionSource};

pub fn configured_event_count(app: &App) -> usize {
    action_entries_for_view(app).len()
}

pub(super) fn action_entries_for_view(app: &App) -> Vec<ActionEntry> {
    match app.actions_view {
        ActionView::Effective => effective_action_entries(app),
        ActionView::User => action_entries_from_map(&app.config.actions, ActionSource::User),
        ActionView::Repo => app
            .repo_config
            .as_ref()
            .map(|repo| action_entries_from_map(&repo.actions, ActionSource::Repo))
            .unwrap_or_default(),
    }
}

fn action_entries_from_map(
    actions: &HashMap<AgenticEvent, Vec<HookAction>>,
    source: ActionSource,
) -> Vec<ActionEntry> {
    let mut entries: Vec<_> = actions
        .iter()
        .filter(|(_, actions)| !actions.is_empty())
        .map(|(event, actions)| ActionEntry {
            event: *event,
            actions: actions.clone(),
            source,
        })
        .collect();
    entries.sort_by_key(|entry| entry.event.as_slug());
    entries
}

fn effective_action_entries(app: &App) -> Vec<ActionEntry> {
    let mut merged = action_entries_from_map(&app.config.actions, ActionSource::User);
    if let Some(repo) = &app.repo_config {
        for (event, actions) in &repo.actions {
            if actions.is_empty() {
                continue;
            }

            if let Some(existing) = merged.iter_mut().find(|entry| entry.event == *event) {
                existing.actions = actions.clone();
                existing.source = ActionSource::Repo;
            } else {
                merged.push(ActionEntry {
                    event: *event,
                    actions: actions.clone(),
                    source: ActionSource::Repo,
                });
            }
        }
    }
    merged.sort_by_key(|entry| entry.event.as_slug());
    merged
}

pub(super) fn current_actions_map(app: &App) -> Option<&HashMap<AgenticEvent, Vec<HookAction>>> {
    match app.actions_view {
        ActionView::Effective => None,
        ActionView::User => Some(&app.config.actions),
        ActionView::Repo => app.repo_config.as_ref().map(|repo| &repo.actions),
    }
}

pub(super) fn current_actions_map_mut(
    app: &mut App,
) -> Option<&mut HashMap<AgenticEvent, Vec<HookAction>>> {
    match app.actions_view {
        ActionView::Effective => None,
        ActionView::User => Some(&mut app.config.actions),
        ActionView::Repo => {
            if app.repo_config.is_none() {
                app.repo_config =
                    Some(claudine::config::claudine_config::RepoOverrideConfig::default());
            }
            app.repo_config.as_mut().map(|repo| &mut repo.actions)
        }
    }
}

pub(super) fn mark_current_actions_dirty(app: &mut App) {
    match app.actions_view {
        ActionView::Effective => {}
        ActionView::User => app.dirty = true,
        ActionView::Repo => app.repo_dirty = true,
    }
}

pub(super) fn switch_effective_selection_to_source_view(app: &mut App) {
    if app.actions_view != ActionView::Effective {
        return;
    }

    let Some(selected) = action_entries_for_view(app).get(app.list_index).cloned() else {
        return;
    };

    app.actions_view = selected.source.view();
    if let Some(index) = action_entries_for_view(app)
        .iter()
        .position(|entry| entry.event == selected.event)
    {
        app.list_index = index;
    }
}

pub(super) fn get_unconfigured_events(app: &App) -> Vec<AgenticEvent> {
    let Some(actions) = current_actions_map(app) else {
        return Vec::new();
    };

    AgenticEvent::ALL
        .into_iter()
        .filter(|event| {
            actions
                .get(event)
                .is_none_or(|configured| configured.is_empty())
        })
        .collect()
}
