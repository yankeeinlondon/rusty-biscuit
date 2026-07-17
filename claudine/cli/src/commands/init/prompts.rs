//! Interactive prompts for the init wizard using the inquire crate.
//!
//! This module is compiled only for tests (`#[cfg(test)] pub mod init;`
//! in `commands/mod.rs`). The production wizard lives in
//! `init_wizard`; the helpers here are retained alongside their
//! tests for reference, so `dead_code` is allowed.
#![allow(dead_code)]

use std::collections::HashSet;
use std::path::PathBuf;

use claudine::actions::{HookAction, LogTarget};
use claudine::events::{
    AgenticEvent, INIT_EVENT_DISPLAY_ORDER, INIT_RECOMMENDED_EVENTS, default_speak_template,
    recommended_sound,
};
use claudine::linking::preference_prompt_count;
use claudine::provider::Provider;
use color_eyre::eyre::{Result, WrapErr};
use inquire::{Confirm, MultiSelect, Select, Text};

/// Prompt user for ranked provider preferences, reusing prior answers as defaults.
pub fn prompt_provider_preferences_with_defaults(
    installed_providers: &[Provider],
    defaults: &[Provider],
) -> Result<Vec<Provider>> {
    let mut installed = installed_providers.to_vec();
    installed.sort_by_key(|provider| provider.to_string());
    installed.dedup();

    let prompt_count = preference_prompt_count(installed.len());
    if prompt_count == 0 {
        return Ok(installed);
    }

    let prompt_labels = [
        "Select your favorite agentic CLI:",
        "Select your second favorite agentic CLI:",
        "Select your third favorite agentic CLI:",
    ];

    let mut remaining = installed;
    let mut ranked = Vec::new();

    for (idx, prompt) in prompt_labels.into_iter().take(prompt_count).enumerate() {
        let options: Vec<String> = remaining
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        let starting_cursor = defaults
            .get(idx)
            .and_then(|provider| {
                options
                    .iter()
                    .position(|option| option == &provider.to_string())
            })
            .unwrap_or(0);
        let selected = Select::new(prompt, options.clone())
            .with_help_message("Used for canonical provider ordering")
            .with_starting_cursor(starting_cursor)
            .prompt()?;
        let index = options
            .iter()
            .position(|option| option == &selected)
            .expect("selected provider exists in options");
        ranked.push(remaining.remove(index));
    }

    Ok(ranked)
}

/// Global init action choices applied across all registered events.
#[derive(Debug, Clone)]
pub struct InitActionProfile {
    /// Logging policy applied across registered events.
    pub logging: LoggingProfile,
    /// Actions for events where the agent asks for human input.
    pub input_required_actions: Vec<HookAction>,
}

/// Logging policy selected during init.
#[derive(Debug, Clone)]
pub enum LoggingProfile {
    /// Do not add logging actions by default.
    None,
    /// Add logging actions to all events.
    All { target: LogTarget },
    /// Add logging actions only to selected events.
    Some {
        target: LogTarget,
        events: HashSet<AgenticEvent>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputActionType {
    SaySomething,
    SoundEffect,
    RunCommand,
}

impl std::fmt::Display for InputActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputActionType::SaySomething => write!(f, "Say Something"),
            InputActionType::SoundEffect => write!(f, "Create a Sound Effect"),
            InputActionType::RunCommand => write!(f, "Run a command"),
        }
    }
}

/// Prompt for global action defaults, reusing prior answers when available.
pub fn prompt_action_profile_with_defaults(
    defaults: Option<&InitActionProfile>,
) -> Result<InitActionProfile> {
    let logging = prompt_logging_profile(defaults.map(|profile| &profile.logging))?;
    let input_required_actions = prompt_input_required_actions(
        defaults.map(|profile| profile.input_required_actions.as_slice()),
    )?;

    Ok(InitActionProfile {
        logging,
        input_required_actions,
    })
}

/// Prompt to enable or disable protect.
pub fn prompt_protect_enabled(default: Option<bool>) -> Result<bool> {
    let enabled = inquire::Confirm::new("Enable Protect? (blocks dangerous commands)")
        .with_default(default.unwrap_or(true))
        .prompt()?;
    Ok(enabled)
}

fn prompt_logging_profile(default: Option<&LoggingProfile>) -> Result<LoggingProfile> {
    let options = vec!["All events", "Some events", "No events"];
    let selected = Select::new("Add logging to all, some, or none of the events?", options)
        .with_starting_cursor(logging_profile_starting_cursor(default))
        .prompt()?;

    Ok(match selected {
        "All events" => LoggingProfile::All {
            target: prompt_log_target(default.and_then(logging_target))?,
        },
        "Some events" => {
            let events = prompt_logging_event_selection(default.and_then(logging_events))?;
            if events.is_empty() {
                LoggingProfile::None
            } else {
                LoggingProfile::Some {
                    target: prompt_log_target(default.and_then(logging_target))?,
                    events,
                }
            }
        }
        _ => LoggingProfile::None,
    })
}

fn prompt_logging_event_selection(
    default: Option<&HashSet<AgenticEvent>>,
) -> Result<HashSet<AgenticEvent>> {
    let options: Vec<String> = INIT_EVENT_DISPLAY_ORDER
        .iter()
        .map(|event| format!("{} - {}", event.as_pascal_case(), event.description()))
        .collect();

    let defaults: Vec<usize> = default
        .map(|events| {
            INIT_EVENT_DISPLAY_ORDER
                .iter()
                .enumerate()
                .filter_map(|(idx, event)| events.contains(event).then_some(idx))
                .collect()
        })
        .unwrap_or_else(|| {
            INIT_EVENT_DISPLAY_ORDER
                .iter()
                .enumerate()
                .filter_map(|(idx, event)| INIT_RECOMMENDED_EVENTS.contains(event).then_some(idx))
                .collect()
        });

    let selected = MultiSelect::new("Select events to log:", options)
        .with_default(&defaults)
        .with_help_message("Space to toggle, Enter to confirm")
        .prompt()?;

    Ok(selected
        .iter()
        .filter_map(|opt| {
            let name = opt.split(" - ").next()?;
            INIT_EVENT_DISPLAY_ORDER
                .iter()
                .find(|event| event.as_pascal_case() == name)
                .copied()
        })
        .collect())
}

fn prompt_log_target(default: Option<&LogTarget>) -> Result<LogTarget> {
    let options = vec![
        "Daily local file (~/.claudine/logs/YYYY-MM-DD.jsonl)",
        "Custom file path",
        "Remote server URL",
    ];

    let selected = Select::new("Where should events be logged?", options)
        .with_starting_cursor(log_target_starting_cursor(default))
        .prompt()?;

    Ok(match selected {
        "Daily local file (~/.claudine/logs/YYYY-MM-DD.jsonl)" => LogTarget::File {
            path: None,
            rotate_daily: true,
        },
        "Custom file path" => {
            let default_path = match default {
                Some(LogTarget::File {
                    path: Some(path),
                    rotate_daily: false,
                }) => path.display().to_string(),
                _ => "~/.claudine/events.jsonl".to_string(),
            };
            let path = Text::new("Enter file path:")
                .with_default(&default_path)
                .prompt()?;
            LogTarget::File {
                path: Some(PathBuf::from(shellexpand::tilde(&path).into_owned())),
                rotate_daily: false,
            }
        }
        "Remote server URL" => {
            let mut prompt =
                Text::new("Enter server URL:").with_placeholder("https://example.com/events");
            if let Some(LogTarget::Server { url, .. }) = default {
                prompt = prompt.with_default(url);
            }
            let url_str = prompt.prompt()?;
            let url = url::Url::parse(&url_str).wrap_err("Invalid URL")?;
            LogTarget::Server {
                url: url.to_string(),
                timeout_ms: 10_000,
                headers: None,
            }
        }
        _ => unreachable!(),
    })
}

fn prompt_input_required_actions(defaults: Option<&[HookAction]>) -> Result<Vec<HookAction>> {
    let options = [
        InputActionType::SaySomething,
        InputActionType::SoundEffect,
        InputActionType::RunCommand,
    ];
    let option_strings: Vec<String> = options
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    let default_indices = input_action_default_indices(defaults).unwrap_or_else(|| {
        vec![
            options
                .iter()
                .position(|action| *action == InputActionType::SoundEffect)
                .unwrap_or(0),
        ]
    });

    let selected = MultiSelect::new(
        "What would you like to do when an agent needs input from you?",
        option_strings,
    )
    .with_default(&default_indices)
    .with_help_message("Space to toggle, Enter to confirm")
    .prompt()?;

    let selected_types: Vec<InputActionType> = selected
        .iter()
        .filter_map(|value| options.iter().find(|option| option.to_string() == *value))
        .copied()
        .collect();

    let mut actions = Vec::new();
    for action_type in selected_types {
        match action_type {
            InputActionType::SaySomething => actions.push(prompt_input_speak_action()?),
            InputActionType::SoundEffect => actions.push(prompt_input_sound_effect()?),
            InputActionType::RunCommand => actions.push(prompt_input_run_action()?),
        }
    }
    Ok(actions)
}

fn prompt_input_speak_action() -> Result<HookAction> {
    let default_message = default_speak_template(&AgenticEvent::HumanInTheLoop);
    let message = Text::new("What should Claudine say when input is needed?")
        .with_default(default_message)
        .with_help_message("Supports {{placeholder}} interpolation")
        .prompt()?;
    Ok(HookAction::Speak {
        message,
        voice: None,
        gender: None,
        when: None,
    })
}

fn prompt_input_sound_effect() -> Result<HookAction> {
    let recommended = recommended_sound(&AgenticEvent::HumanInTheLoop);
    let effects = playa::SoundEffect::all();
    let names: Vec<&str> = effects.iter().map(|effect| effect.name()).collect();
    let options: Vec<String> = names
        .iter()
        .map(|name| {
            if *name == recommended {
                format!("{name} (recommended)")
            } else {
                name.to_string()
            }
        })
        .collect();
    let default_idx = names
        .iter()
        .position(|name| *name == recommended)
        .unwrap_or(0);
    let selected = Select::new("Select a sound effect for input-needed events:", options)
        .with_starting_cursor(default_idx)
        .prompt()?;

    Ok(HookAction::SoundEffect {
        effect: selected.replace(" (recommended)", ""),
        volume: 1.0,
        speed: 1.0,
        when: None,
    })
}

fn prompt_input_run_action() -> Result<HookAction> {
    let command = Text::new("Command to run when input is needed:")
        .with_placeholder("notify-send")
        .prompt()?;
    let params_str = Text::new("Parameters (template-interpolated, or leave empty):")
        .with_default("")
        .prompt()?;
    let blocking = Confirm::new("Wait for command to complete?")
        .with_default(false)
        .prompt()?;

    if blocking {
        let args = if params_str.is_empty() {
            None
        } else {
            Some(params_str.split_whitespace().map(String::from).collect())
        };
        Ok(HookAction::Call {
            command,
            args,
            timeout_ms: None,
            mapper: None,
            when: None,
        })
    } else {
        Ok(HookAction::Bash {
            command,
            params: params_str,
            when: None,
        })
    }
}

/// Gitignore handling options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GitignoreChoice {
    AddToGitignore,
    CommitIt,
    DoNothing,
}

/// Prompt for .gitignore handling in repo mode.
pub fn prompt_gitignore_choice() -> Result<GitignoreChoice> {
    let options = vec![
        "Add .claudine/ to .gitignore",
        "Commit .claudine/config.json",
        "Do nothing",
    ];

    let selected = Select::new("How should .claudine config be handled?", options).prompt()?;

    Ok(match selected {
        "Add .claudine/ to .gitignore" => GitignoreChoice::AddToGitignore,
        "Commit .claudine/config.json" => GitignoreChoice::CommitIt,
        _ => GitignoreChoice::DoNothing,
    })
}

fn logging_profile_starting_cursor(default: Option<&LoggingProfile>) -> usize {
    match default {
        Some(LoggingProfile::All { .. }) => 0,
        Some(LoggingProfile::Some { .. }) => 1,
        Some(LoggingProfile::None) => 2,
        None => 0,
    }
}

fn logging_target(default: &LoggingProfile) -> Option<&LogTarget> {
    match default {
        LoggingProfile::All { target } | LoggingProfile::Some { target, .. } => Some(target),
        LoggingProfile::None => None,
    }
}

fn logging_events(default: &LoggingProfile) -> Option<&HashSet<AgenticEvent>> {
    match default {
        LoggingProfile::Some { events, .. } => Some(events),
        _ => None,
    }
}

fn log_target_starting_cursor(default: Option<&LogTarget>) -> usize {
    match default {
        Some(LogTarget::File {
            path: None,
            rotate_daily: true,
        }) => 0,
        Some(LogTarget::File {
            path: Some(_),
            rotate_daily: false,
        }) => 1,
        Some(LogTarget::Server { .. }) => 2,
        _ => 0,
    }
}

fn input_action_default_indices(defaults: Option<&[HookAction]>) -> Option<Vec<usize>> {
    let defaults = defaults?;
    let mut indices = Vec::new();
    for action in defaults {
        let index = match action {
            HookAction::Speak { .. } => 0,
            HookAction::SoundEffect { .. } => 1,
            HookAction::Bash { .. } | HookAction::Call { .. } => 2,
            _ => continue,
        };
        if !indices.contains(&index) {
            indices.push(index);
        }
    }
    Some(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logging_defaults_to_all_events_for_new_init() {
        assert_eq!(logging_profile_starting_cursor(None), 0);
    }

    #[test]
    fn logging_profile_cursor_reuses_previous_answer() {
        assert_eq!(
            logging_profile_starting_cursor(Some(&LoggingProfile::Some {
                target: LogTarget::File {
                    path: None,
                    rotate_daily: true,
                },
                events: HashSet::new(),
            })),
            1
        );
        assert_eq!(
            logging_profile_starting_cursor(Some(&LoggingProfile::None)),
            2
        );
    }

    #[test]
    fn input_actions_defaults_follow_existing_actions() {
        let defaults = vec![
            HookAction::Speak {
                message: "hello".to_string(),
                voice: None,
                gender: None,
                when: None,
            },
            HookAction::SoundEffect {
                effect: "ding".to_string(),
                volume: 1.0,
                speed: 1.0,
                when: None,
            },
            HookAction::Bash {
                command: "notify-send".to_string(),
                params: String::new(),
                when: None,
            },
        ];

        assert_eq!(
            input_action_default_indices(Some(&defaults)),
            Some(vec![0, 1, 2])
        );
    }
}
