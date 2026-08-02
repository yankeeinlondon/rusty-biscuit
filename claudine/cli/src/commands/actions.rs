use std::collections::BTreeMap;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::actions::HookAction;
use claudine::config::claudine_config::ClaudineConfig;
use claudine::dispatch::loader::load_claudine_config;

use crate::cli_utils::event_name_pascal;
use crate::log;

#[derive(Args)]
pub struct ActionsArgs {}

pub fn run(_args: ActionsArgs, verbose: bool) -> Result<()> {
    let config = match load_claudine_config(None, None) {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error(&format!("Failed to load config: {}", e));
            return Ok(());
        }
    };

    let term = crate::log::terminal();

    if verbose {
        run_verbose(&config, &term)
    } else {
        run_simple(&config, &term)
    }
}

fn action_type_name(action: &HookAction) -> &'static str {
    action.type_pascal_case()
}

fn run_simple(config: &ClaudineConfig, term: &Terminal) -> Result<()> {
    let Some(rendered) = render_simple(config, term) else {
        log::data("");
        log::data("No configured actions found.");
        log::data("");
        log::data(
            &Prose::new("<dim>Run <blue>claudine config</blue> to configure event hooks.</dim>")
                .render(term),
        );
        return Ok(());
    };

    log::data(&format!("\n{}", rendered));

    Ok(())
}

fn render_simple(config: &ClaudineConfig, term: &Terminal) -> Option<String> {
    let mut action_to_events: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (event, actions) in &config.actions {
        for action in actions {
            let action_name = action_type_name(action);
            action_to_events
                .entry(action_name.to_string())
                .or_default()
                .push(event.as_slug().to_string());
        }
    }

    if action_to_events.is_empty() {
        return None;
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

    Some(table.render(term))
}

fn action_with_params(action: &HookAction) -> String {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
            ..
        } => {
            let mut s = format!("{}({effect}", action.type_pascal_case());
            if *volume != 1.0 {
                s.push_str(&format!(", vol={}", volume));
            }
            if *speed != 1.0 {
                s.push_str(&format!(", speed={}", speed));
            }
            s.push(')');
            s
        }
        HookAction::Speak { message, .. } => {
            let truncated = if message.len() > 30 {
                format!("{}…", &message[..27])
            } else {
                message.clone()
            };
            format!("{}({truncated})", action.type_pascal_case())
        }
        HookAction::Bash {
            command, params, ..
        } => {
            if params.is_empty() {
                format!("{}({})", action.type_pascal_case(), command)
            } else {
                format!("{}({} {})", action.type_pascal_case(), command, params)
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
        HookAction::Report { handler, .. } => {
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

fn run_verbose(config: &ClaudineConfig, term: &Terminal) -> Result<()> {
    let mut action_to_events: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (event, actions) in &config.actions {
        for action in actions {
            let action_key = action_with_params(action);
            action_to_events
                .entry(action_key)
                .or_default()
                .push(event.as_slug().to_string());
        }
    }

    if action_to_events.is_empty() {
        log::data("");
        log::data("No configured actions found.");
        log::data("");
        log::data(
            &Prose::new("<dim>Run <blue>claudine config</blue> to configure event hooks.</dim>")
                .render(term),
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

    let rendered = table.render(term);
    log::data(&format!("\n{}", rendered));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::events::AgenticEvent;

    #[test]
    fn configured_actions_render_session_start_report() {
        let config: ClaudineConfig = serde_json::from_str(
            r#"{
  "actions": {
    "session_start": [
      { "type": "report", "handler": { "format": "json" } }
    ]
  }
}"#,
        )
        .unwrap();
        assert!(config.actions.contains_key(&AgenticEvent::SessionStart));

        let rendered = render_simple(&config, &crate::log::optimistic_terminal(Some(100)))
            .expect("configured actions should render a table");
        assert!(rendered.contains("Report"));
        assert!(rendered.contains("SessionStart"));
    }
}
