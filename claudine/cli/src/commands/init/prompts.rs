//! Interactive prompts for the init wizard using the inquire crate.

use std::path::PathBuf;

use claudine::config::AgentInfo;
use claudine::events::{AgenticEvent, EventAction, GlobalSettings, LogTarget, TtsSettings};
use color_eyre::eyre::Result;
use inquire::{Confirm, MultiSelect, Select, Text};

use super::defaults::{
    all_events_ordered, default_speak_template, event_description, recommended_events,
    recommended_sound, tts_providers,
};

/// Prompt user to select which agents to configure.
///
/// Returns the selected providers. Pre-selects agents that are available.
pub fn prompt_agent_selection(agents: &[AgentInfo]) -> Result<Vec<AgentInfo>> {
    let available: Vec<_> = agents.iter().filter(|a| a.is_available()).collect();
    let unavailable: Vec<_> = agents.iter().filter(|a| !a.is_available()).collect();

    if available.is_empty() {
        eprintln!("\nNo agents detected on your system.");
        eprintln!("Install at least one of: claude, gemini, codex, opencode");
        return Ok(vec![]);
    }

    // Build options with status indicators
    let options: Vec<String> = agents
        .iter()
        .map(|a| {
            let status = if a.config_exists && a.on_path {
                "(config + binary)"
            } else if a.config_exists {
                "(config only)"
            } else if a.on_path {
                "(binary only)"
            } else {
                "(not detected)"
            };
            format!("{} {}", a.display_name, status)
        })
        .collect();

    // Pre-select available agents
    let defaults: Vec<usize> = agents
        .iter()
        .enumerate()
        .filter_map(|(i, a)| if a.is_available() { Some(i) } else { None })
        .collect();

    let selected = MultiSelect::new("Select agents to configure:", options)
        .with_default(&defaults)
        .with_help_message("Space to toggle, Enter to confirm")
        .prompt()?;

    // Map back to AgentInfo
    let result: Vec<AgentInfo> = selected
        .iter()
        .filter_map(|opt| {
            agents
                .iter()
                .find(|a| opt.starts_with(a.display_name))
                .cloned()
        })
        .collect();

    if result.is_empty() && !unavailable.is_empty() {
        eprintln!("\nNo agents selected. At least one agent is required.");
    }

    Ok(result)
}

/// Prompt user to select which events to configure.
///
/// Returns the selected events. Pre-selects recommended events.
pub fn prompt_event_selection() -> Result<Vec<AgenticEvent>> {
    let all_events = all_events_ordered();
    let recommended = recommended_events();

    let options: Vec<String> = all_events
        .iter()
        .map(|e| {
            let desc = event_description(e);
            format!("{} - {}", e, desc)
        })
        .collect();

    let defaults: Vec<usize> = all_events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            if recommended.contains(e) {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    let selected = MultiSelect::new("Select events to configure:", options)
        .with_default(&defaults)
        .with_help_message("Recommended events are pre-selected")
        .prompt()?;

    // Map back to AgenticEvent
    let result: Vec<AgenticEvent> = selected
        .iter()
        .filter_map(|opt| {
            let event_name = opt.split(" - ").next()?;
            all_events
                .iter()
                .find(|e| e.to_string() == event_name)
                .cloned()
        })
        .collect();

    Ok(result)
}

/// Action types available for configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionType {
    SoundEffect,
    Speak,
    Log,
    Report,
    Run,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::SoundEffect => write!(f, "Sound Effect"),
            ActionType::Speak => write!(f, "Text-to-Speech"),
            ActionType::Log => write!(f, "Log to File"),
            ActionType::Report => write!(f, "Report to Output"),
            ActionType::Run => write!(f, "Run Command"),
        }
    }
}

/// Prompt user to select action types for an event.
pub fn prompt_action_types(event: &AgenticEvent) -> Result<Vec<ActionType>> {
    let options = vec![
        ActionType::SoundEffect,
        ActionType::Speak,
        ActionType::Log,
        ActionType::Report,
        ActionType::Run,
    ];

    let option_strings: Vec<String> = options.iter().map(|a| a.to_string()).collect();

    // Default to SoundEffect for most events
    let defaults = vec![0]; // SoundEffect

    let selected = MultiSelect::new(&format!("Actions for {} event:", event), option_strings)
        .with_default(&defaults)
        .with_help_message("Select one or more actions")
        .prompt()?;

    let result: Vec<ActionType> = selected
        .iter()
        .filter_map(|s| options.iter().find(|a| &a.to_string() == s).copied())
        .collect();

    Ok(result)
}

/// Prompt user to select a sound effect for an event.
pub fn prompt_sound_effect(event: &AgenticEvent) -> Result<EventAction> {
    let recommended = recommended_sound(event);

    // Common sound effects from playa
    let sounds = vec![
        "success",
        "error",
        "notification",
        "power-up",
        "power-down",
        "high-up",
        "phase-jump-1",
        "electronic-hit-fx-01",
        "dit-hit-1",
        "dit-hit-2",
        "sad-trombone",
        "doorbell",
        "quick-blip-1",
        "sending",
        "receiving",
        "swoosh",
    ];

    let options: Vec<String> = sounds
        .iter()
        .map(|s| {
            if *s == recommended {
                format!("{} (recommended)", s)
            } else {
                s.to_string()
            }
        })
        .collect();

    // Find recommended index
    let default_idx = sounds.iter().position(|s| *s == recommended).unwrap_or(0);

    let selected = Select::new(&format!("Sound for {} event:", event), options)
        .with_starting_cursor(default_idx)
        .prompt()?;

    let name = selected.replace(" (recommended)", "").to_string();

    Ok(EventAction::SoundEffect {
        name,
        volume: 1.0,
        speed: 1.0,
    })
}

/// Prompt user to configure a speak action for an event.
pub fn prompt_speak_action(event: &AgenticEvent) -> Result<EventAction> {
    let default_template = default_speak_template(event);

    let message = Text::new(&format!("Speak message for {} event:", event))
        .with_default(default_template)
        .with_help_message("Supports {placeholder} interpolation")
        .prompt()?;

    Ok(EventAction::Speak { message })
}

/// Prompt user to configure a log action.
pub fn prompt_log_action() -> Result<EventAction> {
    let options = vec![
        "Local file (~/.claudine/events.jsonl)",
        "Custom file path",
        "Remote server URL",
    ];

    let selected = Select::new("Log destination:", options).prompt()?;

    let target = match selected {
        "Local file (~/.claudine/events.jsonl)" => {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
            LogTarget::LocalFile {
                path: home.join(".claudine").join("events.jsonl"),
            }
        }
        "Custom file path" => {
            let path = Text::new("Enter file path:")
                .with_default("~/.claudine/events.jsonl")
                .prompt()?;
            LogTarget::LocalFile {
                path: PathBuf::from(shellexpand::tilde(&path).into_owned()),
            }
        }
        "Remote server URL" => {
            let url_str = Text::new("Enter server URL:")
                .with_placeholder("https://example.com/events")
                .prompt()?;
            let url = url::Url::parse(&url_str)
                .map_err(|e| color_eyre::eyre::eyre!("Invalid URL: {}", e))?;
            LogTarget::Server { url }
        }
        _ => unreachable!(),
    };

    Ok(EventAction::Log { target })
}

/// Prompt user to configure a report action.
pub fn prompt_report_action() -> Result<EventAction> {
    // For simplicity, use default reporter (no custom handler)
    Ok(EventAction::Report { handler: None })
}

/// Prompt user to configure a run command action.
pub fn prompt_run_action(event: &AgenticEvent) -> Result<EventAction> {
    let command = Text::new(&format!("Command to run on {} event:", event))
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

    Ok(EventAction::Run {
        command,
        args,
        blocking,
    })
}

/// Configure all actions for an event.
pub fn configure_event_actions(event: &AgenticEvent) -> Result<Vec<EventAction>> {
    let action_types = prompt_action_types(event)?;

    if action_types.is_empty() {
        return Ok(vec![]);
    }

    let mut actions = Vec::new();

    for action_type in action_types {
        let action = match action_type {
            ActionType::SoundEffect => prompt_sound_effect(event)?,
            ActionType::Speak => prompt_speak_action(event)?,
            ActionType::Log => prompt_log_action()?,
            ActionType::Report => prompt_report_action()?,
            ActionType::Run => prompt_run_action(event)?,
        };
        actions.push(action);
    }

    Ok(actions)
}

/// Prompt user to configure global TTS settings.
pub fn prompt_tts_settings() -> Result<Option<TtsSettings>> {
    let configure_tts = Confirm::new("Configure text-to-speech settings?")
        .with_default(false)
        .prompt()?;

    if !configure_tts {
        return Ok(None);
    }

    let providers = tts_providers();
    let options: Vec<String> = providers.iter().map(|(_, name)| name.to_string()).collect();

    let selected = Select::new("TTS provider:", options).prompt()?;

    let provider = providers
        .iter()
        .find(|(_, name)| name == &selected)
        .map(|(id, _)| id.to_string());

    let voice = Text::new("Voice name (or leave empty for default):")
        .with_default("")
        .prompt()?;

    let rate_str = Text::new("Speech rate (1.0 = normal, 1.5 = faster):")
        .with_default("1.0")
        .prompt()?;

    let rate = rate_str.parse::<f32>().ok();

    Ok(Some(TtsSettings {
        provider,
        voice: if voice.is_empty() { None } else { Some(voice) },
        rate,
    }))
}

/// Prompt user to configure global settings.
pub fn prompt_global_settings() -> Result<GlobalSettings> {
    let tts = prompt_tts_settings()?;

    // Ask about default log target
    let configure_log = Confirm::new("Set a default log target for all events?")
        .with_default(false)
        .prompt()?;

    let default_log_target = if configure_log {
        Some(match prompt_log_action()? {
            EventAction::Log { target } => target,
            _ => unreachable!(),
        })
    } else {
        None
    };

    Ok(GlobalSettings {
        default_log_target,
        tts,
    })
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
        "Add .hooker to .gitignore",
        "Commit .hooker to the repository",
        "Do nothing",
    ];

    let selected = Select::new("How should .hooker be handled?", options).prompt()?;

    Ok(match selected {
        "Add .hooker to .gitignore" => GitignoreChoice::AddToGitignore,
        "Commit .hooker to the repository" => GitignoreChoice::CommitIt,
        _ => GitignoreChoice::DoNothing,
    })
}
