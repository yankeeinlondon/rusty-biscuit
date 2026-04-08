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

use crate::commands::config_tui::app::App;

#[derive(Debug, Args)]
pub struct ConfigArgs {}

pub async fn run(_args: ConfigArgs) -> color_eyre::Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    if !config_path.exists() {
        return super::init_wizard::run_initialization().await;
    }

    let config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)?;
    let cwd = std::env::current_dir()?;
    let git_info = sniff::filesystem::git::detect_git(&cwd, false, 1).ok().flatten();
    let is_in_repo = git_info.is_some();

    let (repo_config, repo_config_path) = if let Some(ref git) = git_info {
        let repo_root = &git.repo_root;
        let repo_cfg_path = repo_root.join(".claudine").join("config.json");
        let repo_cfg = if repo_cfg_path.exists() {
            claudine::dispatch::loader::load_claudine_config(Some(&repo_cfg_path), None).ok()
        } else {
            None
        };
        (repo_cfg, Some(repo_cfg_path))
    } else {
        (None, None)
    };

    let mut app = App::new(config, repo_config, repo_config_path, is_in_repo);

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

    if app.repo_dirty {
        if let Some(ref path) = app.repo_config_path {
            if let Some(ref repo_cfg) = app.repo_config {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                claudine::dispatch::loader::save_claudine_config(repo_cfg, path)?;
                eprintln!("Repo configuration saved to {}", path.display());
            }
        }
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

fn get_available_providers() -> Vec<claudine::events::Provider> {
    let agents = claudine::config::discover_agents_full();
    agents
        .iter()
        .filter(|a| a.is_available())
        .map(|a| a.provider)
        .collect()
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

pub fn query_voices_for_provider(provider: &str) -> Vec<String> {
    match provider {
        "say" => query_say_voices(),
        "espeak-ng" => query_espeak_voices("espeak-ng"),
        "espeak" => query_espeak_voices("espeak"),
        _ => vec![],
    }
}

fn query_say_voices() -> Vec<String> {
    let output = match std::process::Command::new("say").arg("-v").arg("?").output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    if !output.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            Some(name.to_string())
        })
        .collect()
}

fn query_espeak_voices(binary: &str) -> Vec<String> {
    let output = match std::process::Command::new(binary).arg("--voices").output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    if !output.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1) // header line
        .filter_map(|line| {
            // espeak --voices format: Pty Language Age/Gender VoiceName ...
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.get(3).map(|s| s.to_string())
        })
        .collect()
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
