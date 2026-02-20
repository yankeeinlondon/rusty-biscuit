use std::collections::BTreeMap;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::actions::HookAction;
use claudine::dispatch::loader::load_config;
use claudine::events::AgenticEvent;

use crate::log;

#[derive(Args)]
pub struct ActionsArgs {}

pub fn run(_args: ActionsArgs, verbose: bool) -> Result<()> {
    let config = match load_config(None, None) {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error(&format!("Failed to load config: {}", e));
            return Ok(());
        }
    };

    let term = Terminal::new();

    if verbose {
        run_verbose(&config, &term)
    } else {
        run_simple(&config, &term)
    }
}

fn action_type_name(action: &HookAction) -> &'static str {
    action.type_pascal_case()
}

fn event_name_pascal(slug: &str) -> String {
    AgenticEvent::from_slug(slug)
        .map(|event| event.as_pascal_case().to_string())
        .unwrap_or_else(|| slug.to_string())
}

fn run_simple(config: &claudine::events::HookerConfig, term: &Terminal) -> Result<()> {
    let mut action_to_events: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (_provider, provider_config) in &config.providers {
        for (event, binding) in &provider_config.events {
            if !binding.enabled {
                continue;
            }

            for action in &binding.actions {
                let action_name = action_type_name(action);
                action_to_events
                    .entry(action_name.to_string())
                    .or_default()
                    .push(event.as_slug().to_string());
            }
        }
    }

    if action_to_events.is_empty() {
        log::data("");
        log::data("No configured actions found.");
        log::data("");
        log::data(
            "{{dim}}Run {{blue}}claudine init{{reset}}{{dim}} to configure event hooks.{{reset}}",
        );
        return Ok(());
    }

    let columns = vec![
        TableColumn::new("Action").with_min_width(10),
        TableColumn::new("Events"),
    ];
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

    for (action_name, events) in &action_to_events {
        let mut unique_events: Vec<String> = events.clone();
        unique_events.sort();
        unique_events.dedup();

        let events_str = unique_events
            .iter()
            .map(|event| event_name_pascal(event))
            .collect::<Vec<_>>()
            .join(", ");
        table.add_row(vec![action_name.clone().into(), events_str.into()]);
    }

    let rendered = table.fallback_render(term);
    log::data(&format!("\n{}", rendered));

    Ok(())
}

fn action_with_params(action: &HookAction) -> String {
    match action {
        HookAction::SoundEffect {
            name,
            volume,
            speed,
        } => {
            let mut s = format!("{}({name}", action.type_pascal_case());
            if *volume != 1.0 {
                s.push_str(&format!(", vol={}", volume));
            }
            if *speed != 1.0 {
                s.push_str(&format!(", speed={}", speed));
            }
            s.push(')');
            s
        }
        HookAction::Speak { message } => {
            let truncated = if message.len() > 30 {
                format!("{}…", &message[..27])
            } else {
                message.clone()
            };
            format!("{}({truncated})", action.type_pascal_case())
        }
        HookAction::Log { target } => {
            let params = match target {
                claudine::actions::LogTarget::File { path, rotate_daily } => {
                    let mut p = String::new();
                    if *rotate_daily != true {
                        p.push_str(&format!("rotate_daily: {}", rotate_daily));
                    }
                    if path.is_some() {
                        if !p.is_empty() {
                            p.push_str(", ");
                        }
                        p.push_str("path: ...");
                    }
                    p
                }
                claudine::actions::LogTarget::Server {
                    url, timeout_ms, ..
                } => {
                    let mut p = format!("url: {}", url);
                    if *timeout_ms != 10_000 {
                        p.push_str(&format!(", timeout: {}ms", timeout_ms));
                    }
                    p
                }
                _ => String::new(),
            };
            if params.is_empty() {
                format!("{}()", action.type_pascal_case())
            } else {
                format!("{}({{{params}}})", action.type_pascal_case())
            }
        }
        HookAction::FireAndForget { command, args } => {
            if let Some(args) = args {
                format!(
                    "{}({} {})",
                    action.type_pascal_case(),
                    command,
                    args.join(" ")
                )
            } else {
                format!("{}({})", action.type_pascal_case(), command)
            }
        }
        HookAction::Call {
            command,
            args,
            timeout_ms,
            ..
        } => {
            let mut s = format!("{}({}", action.type_pascal_case(), command);
            if let Some(args) = args {
                s.push_str(&format!(" {}", args.join(" ")));
            }
            if let Some(timeout) = timeout_ms {
                s.push_str(&format!(", timeout={}ms", timeout));
            }
            s.push(')');
            s
        }
        HookAction::Report { handler } => {
            if let Some(h) = handler {
                let mut params = format!("format: {:?}", h.format);
                if h.include_metadata {
                    params.push_str(", include_metadata: true");
                }
                if let Some(template) = &h.template {
                    params.push_str(&format!(
                        ", template: {}…",
                        &template[..template.len().min(20)]
                    ));
                }
                format!("{}({{{params}}})", action.type_pascal_case())
            } else {
                format!("{}()", action.type_pascal_case())
            }
        }
        _ => action.type_pascal_case().to_string(),
    }
}

fn run_verbose(config: &claudine::events::HookerConfig, term: &Terminal) -> Result<()> {
    let mut action_to_events: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (_provider, provider_config) in &config.providers {
        for (event, binding) in &provider_config.events {
            if !binding.enabled {
                continue;
            }

            for action in &binding.actions {
                let action_key = action_with_params(action);
                action_to_events
                    .entry(action_key)
                    .or_default()
                    .push(event.as_slug().to_string());
            }
        }
    }

    if action_to_events.is_empty() {
        log::data("");
        log::data("No configured actions found.");
        log::data("");
        log::data(
            "{{dim}}Run {{blue}}claudine init{{reset}}{{dim}} to configure event hooks.{{reset}}",
        );
        return Ok(());
    }

    let min_width = 20;
    let columns = vec![
        TableColumn::new("Action")
            .with_min_width(min_width)
            .with_word_wrap(WordWrap::None),
        TableColumn::new("Events"),
    ];
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

    for (action_key, events) in &action_to_events {
        let mut unique_events: Vec<String> = events.clone();
        unique_events.sort();
        unique_events.dedup();

        let events_str = unique_events
            .iter()
            .map(|event| event_name_pascal(event))
            .collect::<Vec<_>>()
            .join(", ");
        table.add_row(vec![action_key.clone().into(), events_str.into()]);
    }

    let rendered = table.fallback_render(term);
    log::data(&format!("\n{}", rendered));

    Ok(())
}
