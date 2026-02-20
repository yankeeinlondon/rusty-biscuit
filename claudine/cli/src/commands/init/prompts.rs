//! Interactive prompts for the init wizard using the inquire crate.

use std::collections::HashSet;
use std::path::PathBuf;

use claudine::actions::{HookAction, LogTarget};
use claudine::events::{
    AgenticEvent, INIT_EVENT_DISPLAY_ORDER, INIT_RECOMMENDED_EVENTS, Provider,
    default_speak_template, recommended_sound,
};
use claudine::linking::preference_prompt_count;
use color_eyre::eyre::Result;
use inquire::{Confirm, MultiSelect, Select, Text};

/// Prompt user for ranked provider preferences based on installed provider count.
///
/// Returns only the explicitly ranked providers. Remaining installed providers
/// should be appended alphabetically by the caller.
pub fn prompt_provider_preferences(installed_providers: &[Provider]) -> Result<Vec<Provider>> {
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

    for prompt in prompt_labels.into_iter().take(prompt_count) {
        let options: Vec<String> = remaining
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        let selected = Select::new(prompt, options.clone())
            .with_help_message("Used for canonical provider ordering")
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

/// Prompt for global action defaults.
pub fn prompt_action_profile() -> Result<InitActionProfile> {
    let logging = prompt_logging_profile()?;
    let input_required_actions = prompt_input_required_actions()?;

    Ok(InitActionProfile {
        logging,
        input_required_actions,
    })
}

fn prompt_logging_profile() -> Result<LoggingProfile> {
    let options = vec!["All events", "Some events", "No events"];
    let selected = Select::new("Add logging to all, some, or none of the events?", options)
        .with_starting_cursor(2)
        .prompt()?;

    Ok(match selected {
        "All events" => LoggingProfile::All {
            target: prompt_log_target()?,
        },
        "Some events" => {
            let events = prompt_logging_event_selection()?;
            if events.is_empty() {
                LoggingProfile::None
            } else {
                LoggingProfile::Some {
                    target: prompt_log_target()?,
                    events,
                }
            }
        }
        _ => LoggingProfile::None,
    })
}

fn prompt_logging_event_selection() -> Result<HashSet<AgenticEvent>> {
    let options: Vec<String> = INIT_EVENT_DISPLAY_ORDER
        .iter()
        .map(|event| format!("{} - {}", event.as_pascal_case(), event.description()))
        .collect();

    let defaults: Vec<usize> = INIT_EVENT_DISPLAY_ORDER
        .iter()
        .enumerate()
        .filter_map(|(idx, event)| INIT_RECOMMENDED_EVENTS.contains(event).then_some(idx))
        .collect();

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

fn prompt_log_target() -> Result<LogTarget> {
    let options = vec![
        "Daily local file (~/.claudine/logs/YYYY-MM-DD.jsonl)",
        "Custom file path",
        "Remote server URL",
    ];

    let selected = Select::new("Where should events be logged?", options).prompt()?;

    Ok(match selected {
        "Daily local file (~/.claudine/logs/YYYY-MM-DD.jsonl)" => LogTarget::File {
            path: None,
            rotate_daily: true,
        },
        "Custom file path" => {
            let path = Text::new("Enter file path:")
                .with_default("~/.claudine/events.jsonl")
                .prompt()?;
            LogTarget::File {
                path: Some(PathBuf::from(shellexpand::tilde(&path).into_owned())),
                rotate_daily: false,
            }
        }
        "Remote server URL" => {
            let url_str = Text::new("Enter server URL:")
                .with_placeholder("https://example.com/events")
                .prompt()?;
            let url = url::Url::parse(&url_str)
                .map_err(|e| color_eyre::eyre::eyre!("Invalid URL: {}", e))?;
            LogTarget::Server {
                url: url.to_string(),
                timeout_ms: 10_000,
                headers: None,
            }
        }
        _ => unreachable!(),
    })
}

fn prompt_input_required_actions() -> Result<Vec<HookAction>> {
    let options = [
        InputActionType::SaySomething,
        InputActionType::SoundEffect,
        InputActionType::RunCommand,
    ];
    let option_strings: Vec<String> = options
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    let default_sound = options
        .iter()
        .position(|action| *action == InputActionType::SoundEffect)
        .unwrap_or(0);

    let selected = MultiSelect::new(
        "What would you like to do when an agent needs input from you?",
        option_strings,
    )
    .with_default(&[default_sound])
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
    Ok(HookAction::Speak { message })
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
        name: selected.replace(" (recommended)", ""),
        volume: 1.0,
        speed: 1.0,
    })
}

fn prompt_input_run_action() -> Result<HookAction> {
    let command = Text::new("Command to run when input is needed:")
        .with_placeholder("notify-send")
        .prompt()?;
    let args_str = Text::new("Arguments (space-separated, or leave empty):")
        .with_default("")
        .prompt()?;
    let args = if args_str.is_empty() {
        None
    } else {
        Some(args_str.split_whitespace().map(String::from).collect())
    };
    let blocking = Confirm::new("Wait for command to complete?")
        .with_default(false)
        .prompt()?;

    if blocking {
        Ok(HookAction::Call {
            command,
            args,
            timeout_ms: None,
            mapper: None,
        })
    } else {
        Ok(HookAction::FireAndForget { command, args })
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
