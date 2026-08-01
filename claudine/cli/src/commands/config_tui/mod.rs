pub mod app;
pub mod reducers;
pub mod tabs;
pub mod widgets;

use std::io::stdout;
use std::time::Duration;

use clap::{Args, Subcommand};
use crossterm::{
    ExecutableCommand,
    event::{self, Event},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui::widgets::*;

use biscuit_speaks::detection::get_available_providers as get_available_tts_providers;
use biscuit_speaks::types::{CloudTtsProvider, HostTtsProvider, TtsProvider};
use claudine::protect::catalog::RuleGroup;
use claudine::protect::config::{ProtectRuleToggles, RuleGroupConfig};
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};

use crate::commands::config_tui::app::{ActionView, App};
use crate::log;

/// Arguments for `claudine config`.
///
/// With no subcommand, launches the interactive TUI. With a subcommand, runs a
/// specific non-interactive setter.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

/// Non-interactive `claudine config <subcommand>` operations.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Set a configuration value.
    Set {
        #[command(subcommand)]
        target: ConfigSetTarget,
    },
}

/// Setter operations for `claudine config set`.
#[derive(Debug, Subcommand)]
pub enum ConfigSetTarget {
    /// Set the favorite agent for lazy composition.
    ///
    /// Pass a provider slug (e.g., `claude`, `codex`, `opencode`) to set the
    /// favorite, or `none`/`clear`/`-` to clear the favorite.
    #[command(name = "favorite-agent")]
    FavoriteAgent {
        /// The provider name (or `none`/`clear`/`-` to clear).
        value: String,
    },
    /// Set whether composition prompts for missing required schema
    /// properties when stdin and stderr are TTYs.
    ///
    /// Pass `true` or `false`.
    #[command(name = "prompt-for-missing")]
    PromptForMissing {
        /// The boolean value (`true` or `false`).
        value: String,
    },
}

pub async fn run(args: ConfigArgs) -> color_eyre::Result<()> {
    if let Some(command) = args.command {
        return run_setter(command).await;
    }
    run_tui().await
}

async fn run_setter(command: ConfigCommand) -> color_eyre::Result<()> {
    match command {
        ConfigCommand::Set { target } => match target {
            ConfigSetTarget::FavoriteAgent { value } => run_set_favorite_agent(&value).await,
            ConfigSetTarget::PromptForMissing { value } => {
                run_set_prompt_for_missing(&value).await
            }
        },
    }
}

async fn run_set_favorite_agent(value: &str) -> color_eyre::Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    let mut config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)?;

    let trimmed = value.trim();
    let new_favorite = match trimmed.to_ascii_lowercase().as_str() {
        "" | "none" | "clear" | "unset" | "-" => None,
        _ => match Provider::fuzzy_match_cli_name(trimmed) {
            Some(provider) => Some(provider),
            None => {
                return Err(color_eyre::eyre::eyre!(
                    "unknown provider '{value}' — try one of: claude, codex, gemini, goose, kimi, opencode, qwen (or pass `none` to clear)"
                ));
            }
        },
    };

    if config.preferred_agent == new_favorite {
        log::message(&match new_favorite {
            Some(provider) => format!("Favorite agent already set to {provider}; no change."),
            None => "Favorite agent already cleared; no change.".to_string(),
        });
        return Ok(());
    }

    config.preferred_agent = new_favorite;
    claudine::dispatch::loader::save_claudine_config(&config, &config_path)?;
    log::message(&match new_favorite {
        Some(provider) => format!("Set favorite agent to {provider}."),
        None => "Cleared favorite agent.".to_string(),
    });
    log::message(&format!(
        "Updated {}.",
        biscuit_file::to_portable_string(&config_path)
    ));
    Ok(())
}

async fn run_set_prompt_for_missing(value: &str) -> color_eyre::Result<()> {
    let new_value = match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        "false" | "0" | "no" | "off" => false,
        _ => {
            return Err(color_eyre::eyre::eyre!(
                "invalid boolean '{value}' — expected `true` or `false`"
            ));
        }
    };

    let config_path = claudine::dispatch::loader::user_config_path();
    let mut config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)?;

    if config.prompt_for_missing == new_value {
        log::message(&format!(
            "prompt_for_missing is already {new_value}; no change."
        ));
        return Ok(());
    }

    config.prompt_for_missing = new_value;
    claudine::dispatch::loader::save_claudine_config(&config, &config_path)?;
    log::message(&format!("Set prompt_for_missing to {new_value}."));
    log::message(&format!(
        "Updated {}.",
        biscuit_file::to_portable_string(&config_path)
    ));
    Ok(())
}

async fn run_tui() -> color_eyre::Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    // Init check is now centralized in main.rs — config is guaranteed to exist here.
    let config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)?;
    let cwd = std::env::current_dir()?;
    let git_info = sniff::filesystem::git::detect_git(&cwd, false, 1)
        .ok()
        .flatten();
    let is_in_repo = git_info.is_some();

    let (repo_config, repo_config_path) = if let Some(ref git) = git_info {
        let repo_root = &git.repo_root;
        let repo_cfg_path = repo_root.join(".claudine").join("config.json");
        let repo_cfg = claudine::dispatch::loader::load_repo_override_config(&repo_cfg_path)
            .ok()
            .flatten();
        (repo_cfg, Some(repo_cfg_path))
    } else {
        (None, None)
    };

    let repo_name = git_info.as_ref().and_then(|g| g.repo.clone());
    let branch_name = git_info.as_ref().and_then(|g| g.current_branch.clone());
    let mut app = App::new(
        config,
        repo_config,
        repo_config_path,
        is_in_repo,
        repo_name,
        branch_name,
    );

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        // Use a short poll timeout so we can check for pending async test
        // results without blocking the event loop indefinitely.
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
        }

        // Poll for pending webhook test-connection results
        if let Some(ref rx) = app.pending_test {
            match rx.try_recv() {
                Ok(result) => {
                    app.pending_test = None;
                    if let Some(app::ModalState::MessengerInput {
                        test_status, error, ..
                    }) = &mut app.modal
                    {
                        *test_status = Some(match result {
                            Ok(()) => "✓ Test connection successful".to_string(),
                            Err(e) => format!("✗ {}", e),
                        });
                        *error = None;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Test still running; leave status as "Testing…"
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Sender dropped without sending; clean up
                    app.pending_test = None;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    if app.dirty || app.repo_dirty {
        if app.dirty {
            claudine::dispatch::loader::save_claudine_config(&app.config, &config_path)?;
        }
        if app.repo_dirty
            && let Some(ref path) = app.repo_config_path
            && let Some(ref repo_cfg) = app.repo_config
        {
            if repo_cfg.is_empty() {
                if path.exists() {
                    std::fs::remove_file(path)?;
                }
            } else {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                claudine::dispatch::loader::save_repo_override_config(repo_cfg, path)?;
            }
        }

        eprintln!();
        eprintln!("\x1b[1mClaudine\x1b[0m configuration was updated:");
        if app.dirty {
            eprintln!(
                "- The \x1b[1mUser\x1b[0m configuration was saved to \x1b[34m~/.claudine/config.json\x1b[0m"
            );
        }
        if app.repo_dirty
            && let (Some(name), Some(branch)) = (&app.repo_name, &app.branch_name)
        {
            eprintln!(
                "- The \x1b[33m{name}\x1b[0m(\x1b[2m{branch}\x1b[0m) \x1b[3mrepo configuration\x1b[0m was saved to \x1b[34m./.claudine/config.json\x1b[0m"
            );
        }
        eprintln!();
    } else {
        eprintln!();
        eprintln!("No changes were made to the \x1b[1mClaudine\x1b[0m configuration.");
        eprintln!("If you want to view the configuration, they are located at:");
        eprintln!(
            "    - \x1b[1mUser\x1b[0m configuration is found in \x1b[34m~/.claudine/config.json\x1b[0m"
        );
        if app.is_in_repo {
            eprintln!(
                "    - \x1b[1mRepo\x1b[0m config is found at \x1b[34m./.claudine/config.json\x1b[0m off the repo's root directory"
            );
        } else {
            eprintln!(
                "    \x1b[2m\x1b[3m- \x1b[1mRepo\x1b[0m\x1b[2m\x1b[3m config is found at \x1b[34m./.claudine/config.json\x1b[0m\x1b[2m\x1b[3m off the repo's root directory\x1b[0m"
            );
            eprintln!(
                "    \x1b[2m\x1b[3m- because you are not in a repo currently no repo based configuration options were presented\x1b[0m"
            );
        }
        eprintln!();
    }
    if app.repo_dirty
        && let Some(ref path) = app.repo_config_path
        && let Some(ref repo_cfg) = app.repo_config
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        claudine::dispatch::loader::save_repo_override_config(repo_cfg, path)?;
        eprintln!(
            "Repo configuration saved to {}",
            biscuit_file::to_portable_string(path)
        );
    }

    Ok(())
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Two-chunk layout: tab bar + content block (hotkeys are inside the content block)
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

    let content_block =
        Block::default()
            .borders(Borders::ALL)
            .border_style(if app.mode == app::AppMode::Detail {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

    let inner = content_block.inner(chunks[1]);
    frame.render_widget(content_block, chunks[1]);

    // Split inner into: tab content area + hotkey bar at bottom
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // Apply left padding of 1 space to the content area
    let content_area = Rect {
        x: inner_chunks[0].x + 1,
        width: inner_chunks[0].width.saturating_sub(1),
        ..inner_chunks[0]
    };

    match app.focused_tab {
        app::Tab::Preferences => tabs::preferences::render(frame, content_area, app),
        app::Tab::Services => tabs::services::render(frame, content_area, app),
        app::Tab::Tts => tabs::tts::render(frame, content_area, app),
        app::Tab::Messenger => tabs::messenger::render(frame, content_area, app),
        app::Tab::Actions => tabs::actions::render(frame, content_area, app),
    }

    // Render centered hotkey bar inside the content block with background
    let hotkey_pairs = build_hotkey_pairs(app);
    let hotkey_line = build_hotkey_line(&hotkey_pairs);
    let hotkey_bg = Block::default().style(Style::default().bg(Color::Indexed(236)));
    frame.render_widget(hotkey_bg, inner_chunks[1]);
    let hotkey_bar = Paragraph::new(hotkey_line)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Indexed(236)));
    frame.render_widget(hotkey_bar, inner_chunks[1]);
}

/// Build the list of (key, description) pairs for the current state.
fn build_hotkey_pairs(app: &App) -> Vec<(&'static str, &'static str)> {
    if app.mode == app::AppMode::Overview {
        return vec![("Enter", "Configure"), ("Q", "Quit")];
    }

    let mut pairs: Vec<(&str, &str)> = vec![("Esc", "Back")];

    match app.focused_tab {
        app::Tab::Preferences => {
            pairs.extend([
                ("A", "Agent"),
                ("U", "User Provider"),
                ("R", "Repo Provider"),
                ("S", "Success"),
                ("N", "Attention"),
                ("E", "Error"),
                ("M", "Prompt For Missing"),
            ]);
        }
        app::Tab::Services => {
            pairs.extend([("L", "Logging"), ("P", "Protect"), ("C", "Configure Rules")]);
        }
        app::Tab::Tts => {
            pairs.extend([
                ("T", "Toggle TTS"),
                ("P", "Provider"),
                ("f", "Female Voice"),
                ("m", "Male Voice"),
                ("F", "Set Female"),
                ("M", "Set Male"),
            ]);
        }
        app::Tab::Actions => {
            if app.is_in_repo {
                pairs.extend([("U", "User"), ("R", "Repo"), ("V", "Effective")]);
            }

            if app.is_in_repo && app.actions_view == ActionView::Effective {
                pairs.push(("ENTER/E", "Edit Source"));
            } else {
                let configured_count = tabs::actions::configured_event_count(app);
                pairs.push(("A", "Add Event"));
                if configured_count > 0 {
                    pairs.extend([("ENTER/E", "Edit"), ("D", "Delete")]);
                }
            }
        }
        app::Tab::Messenger => {
            // T: Test is intentionally omitted from the outer strip because
            // it is only meaningful inside the webhook input modal. The modal
            // itself surfaces the T hotkey when appropriate.
            pairs.extend([
                ("Tab", "Focus"),
                ("Enter", "Activate"),
                ("S", "Select"),
                ("A", "Add"),
            ]);
        }
    }

    pairs
}

/// Build a styled Line from hotkey pairs: keys are bold+yellow, descriptions are light gray.
fn build_hotkey_line(pairs: &[(&str, &str)]) -> Line<'static> {
    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::Indexed(250));
    let sep_style = Style::default().fg(Color::Indexed(240));

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", sep_style));
        }
        spans.push(Span::styled((*key).to_string(), key_style));
        spans.push(Span::styled(format!(": {desc}"), desc_style));
    }
    Line::from(spans)
}

fn get_provider_list() -> Vec<claudine::provider::Provider> {
    PROVIDERS_DISPLAY_ORDER.to_vec()
}

fn get_available_providers(app: &App) -> Vec<claudine::provider::Provider> {
    app.available_providers()
}

fn get_sound_effect_names() -> Vec<&'static str> {
    playa::SoundEffect::all_names().to_vec()
}

/// Get available TTS providers using biscuit-speaks detection (same as `so-you-say list-providers`).
fn get_tts_provider_list() -> Vec<TtsProvider> {
    get_available_tts_providers().to_vec()
}

/// Get the display name for a TTS provider.
pub fn tts_provider_display_name(provider: &TtsProvider) -> &'static str {
    match provider {
        TtsProvider::Host(h) => match h {
            HostTtsProvider::Say => "say (macOS)",
            HostTtsProvider::ESpeak => "espeak (eSpeak-NG)",
            HostTtsProvider::Piper => "piper (Piper TTS)",
            HostTtsProvider::EchoGarden => "echogarden",
            HostTtsProvider::Sherpa => "sherpa (Sherpa-ONNX)",
            HostTtsProvider::Mimic3 => "mimic3 (Mycroft)",
            HostTtsProvider::Festival => "festival",
            HostTtsProvider::Gtts => "gtts (Google TTS CLI)",
            HostTtsProvider::Sapi => "sapi (Windows)",
            HostTtsProvider::KokoroTts => "kokoro (Kokoro TTS)",
            HostTtsProvider::Pico2Wave => "pico2wave",
            HostTtsProvider::SpdSay => "spd-say (Speech Dispatcher)",
            _ => "unknown",
        },
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => "elevenlabs (ElevenLabs API)",
        _ => "unknown",
    }
}

/// Get the short slug name for a TTS provider (used in config files).
pub fn tts_provider_slug(provider: &TtsProvider) -> &'static str {
    match provider {
        TtsProvider::Host(h) => match h {
            HostTtsProvider::Say => "say",
            HostTtsProvider::ESpeak => "espeak",
            HostTtsProvider::Piper => "piper",
            HostTtsProvider::EchoGarden => "echogarden",
            HostTtsProvider::Sherpa => "sherpa",
            HostTtsProvider::Mimic3 => "mimic3",
            HostTtsProvider::Festival => "festival",
            HostTtsProvider::Gtts => "gtts",
            HostTtsProvider::Sapi => "sapi",
            HostTtsProvider::KokoroTts => "kokoro",
            HostTtsProvider::Pico2Wave => "pico2wave",
            HostTtsProvider::SpdSay => "spd-say",
            _ => "unknown",
        },
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => "elevenlabs",
        _ => "unknown",
    }
}

/// Resolve a TTS provider slug back to a TtsProvider enum.
pub fn tts_provider_from_slug(slug: &str) -> Option<TtsProvider> {
    biscuit_speaks::detection::parse_provider_name(slug)
}

pub fn query_voices_for_provider(provider: &str) -> Vec<(String, biscuit_speaks::VoiceQuality)> {
    let tts_provider = tts_provider_from_slug(provider);
    let base_quality = tts_provider
        .as_ref()
        .map(biscuit_speaks::provider_base_quality)
        .unwrap_or(biscuit_speaks::VoiceQuality::Unknown);

    let names: Vec<String> = match provider {
        "say" | "macos" => query_say_voices(),
        "espeak-ng" | "espeak" => query_espeak_voices("espeak-ng"),
        "kokoro" => vec![
            "af_heart",
            "af_bella",
            "af_nicole",
            "af_sarah",
            "af_sky",
            "am_adam",
            "am_michael",
            "bf_emma",
            "bf_isabella",
            "bm_george",
            "bm_lewis",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        _ => vec![],
    };

    names.into_iter().map(|name| (name, base_quality)).collect()
}

fn query_say_voices() -> Vec<String> {
    let output = match std::process::Command::new("say")
        .arg("-v")
        .arg("?")
        .output()
    {
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
