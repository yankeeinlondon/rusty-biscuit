pub mod app;
pub mod tabs;
pub mod widgets;

use std::io::stdout;

use clap::Args;
use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui::widgets::*;

use claudine::events::PROVIDERS_DISPLAY_ORDER;
use claudine::services::protect::config::{
    ProtectRuleToggles, RuleGroupConfig,
};
use claudine::services::protect::catalog::RuleGroup;

use crate::commands::config_tui::app::{App, AppMode};

#[derive(Debug, Args)]
pub struct ConfigArgs {}

pub async fn run(_args: ConfigArgs) -> color_eyre::Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    if !config_path.exists() {
        return super::init_wizard::run_initialization().await;
    }

    let config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)?;
    let is_in_repo = sniff::filesystem::git::detect_git(&std::env::current_dir()?, false, 1)
        .ok()
        .flatten()
        .is_some();

    let mut app = App::new(config, is_in_repo);

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    if app.dirty {
        claudine::dispatch::loader::save_claudine_config(&app.config, &config_path)?;
        eprintln!("Configuration saved to {}", config_path.display());
    }

    Ok(())
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tab_titles: Vec<Line> = app::Tab::ALL
        .iter()
        .map(|tab| {
            let style = if Some(*tab) == app.selected_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else if *tab == app.focused_tab {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(tab.label(), style))
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Claudine Config "),
        )
        .highlight_style(Style::default().fg(Color::Cyan))
        .select(
            app::Tab::ALL
                .iter()
                .position(|t| *t == app.focused_tab)
                .unwrap(),
        );
    frame.render_widget(tabs, chunks[0]);

    let content_block = Block::default().borders(Borders::ALL).border_style(
        if app.mode == app::AppMode::Detail {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        },
    );

    let inner = content_block.inner(chunks[1]);
    frame.render_widget(content_block, chunks[1]);

    match app.focused_tab {
        app::Tab::Preferences => tabs::preferences::render(frame, inner, app),
        app::Tab::Services => tabs::services::render(frame, inner, app),
        app::Tab::Tts => tabs::tts::render(frame, inner, app),
        app::Tab::Messenger => tabs::messenger::render(frame, inner, app),
        app::Tab::Actions => tabs::actions::render(frame, inner, app),
    }
}

fn get_provider_list() -> Vec<claudine::events::Provider> {
    PROVIDERS_DISPLAY_ORDER.to_vec()
}

fn get_sound_effect_names() -> Vec<&'static str> {
    playa::SoundEffect::all_names().to_vec()
}

fn get_tts_providers() -> Vec<&'static str> {
    let mut providers = vec!["auto"];
    if which::which("say").is_ok() {
        providers.push("say");
    }
    if which::which("espeak-ng").is_ok() {
        providers.push("espeak-ng");
    }
    if which::which("espeak").is_ok() {
        providers.push("espeak");
    }
    providers
}

fn get_tts_voices() -> Vec<&'static str> {
    vec![]
}

fn get_protect_rule_names() -> Vec<&'static str> {
    RuleGroup::all_builtin()
        .iter()
        .map(|g| g.config_key())
        .collect()
}

fn is_protect_rule_enabled(rules: &ProtectRuleToggles, key: &str) -> bool {
    let groups = RuleGroup::all_builtin();
    let group = match groups.iter().find(|g| g.config_key() == key) {
        Some(g) => g,
        None => return false,
    };
    match rules.get(*group) {
        None => true,
        Some(RuleGroupConfig::Toggle(enabled)) => *enabled,
        Some(RuleGroupConfig::Detailed(d)) => d.enabled,
    }
}

fn toggle_protect_rule(rules: &mut ProtectRuleToggles, key: &str) {
    let groups = RuleGroup::all_builtin();
    let group = match groups.iter().find(|g| g.config_key() == key) {
        Some(g) => *g,
        None => return,
    };
    let current = match rules.get(group) {
        None => true,
        Some(RuleGroupConfig::Toggle(enabled)) => *enabled,
        Some(RuleGroupConfig::Detailed(d)) => d.enabled,
    };
    let new_config = RuleGroupConfig::Toggle(!current);
    match group {
        RuleGroup::FilesystemDestruction => rules.filesystem_destruction = Some(new_config),
        RuleGroup::DiskManipulation => rules.disk_manipulation = Some(new_config),
        RuleGroup::RemoteExecution => rules.remote_execution = Some(new_config),
        RuleGroup::GitDestructive => rules.git_destructive = Some(new_config),
        RuleGroup::SystemSabotage => rules.system_sabotage = Some(new_config),
        RuleGroup::NetworkSabotage => rules.network_sabotage = Some(new_config),
        RuleGroup::ContainerCloud => rules.container_cloud = Some(new_config),
        RuleGroup::DatabaseNukes => rules.database_nukes = Some(new_config),
        RuleGroup::ObfuscatedExecution => rules.obfuscated_execution = Some(new_config),
        RuleGroup::PromptInjection => rules.prompt_injection = Some(new_config),
        RuleGroup::CredentialExfiltration => rules.credential_exfiltration = Some(new_config),
        RuleGroup::SensitivePaths => rules.sensitive_paths = Some(new_config),
        RuleGroup::Custom => {}
    }
}
