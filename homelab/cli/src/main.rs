//! Homelab automation CLI.

use biscuit_terminal::prelude::{
    Prose, Renderable, Table, TableCellContent, TableColumn, Terminal, UnorderedList,
};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::CompleteEnv;
use color_eyre::Result;
use homelab::arcam::Arcam;
use homelab::config::{HomeyConfig, parse_host_port};
use homelab::network::Host;
use homelab::sony_receiver::{
    GenericSettingResult, SonyError, SonyReceiver, SonyReceiverEndpoints,
};
use serde_json::json;

const COMPLETIONS_HELP: &str = r#"
SHELL COMPLETIONS

Enable dynamic shell completions for homey.

Examples:
  # Bash - add to ~/.bashrc or ~/.bash_profile
  echo 'source <(COMPLETE=bash homey)' >> ~/.bashrc

  # Zsh - add to ~/.zshrc
  echo 'source <(COMPLETE=zsh homey)' >> ~/.zshrc

  # Fish - add to config
  echo 'COMPLETE=fish homey | source' >> ~/.config/fish/config.fish

  # Disable completions
  COMPLETE=0
"#;

/// Homelab automation CLI.
#[derive(Parser)]
#[command(name = "homey")]
#[command(about = "Homelab automation CLI")]
struct Cli {
    /// Output results as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Arcam PA240/PA410/PA720 amplifier control
    Arcam {
        /// Device name from ~/homey.json
        #[arg(long)]
        name: Option<String>,

        /// Arcam host IP or DNS name (overrides config)
        #[arg(long, env = "ARCAM_AMP")]
        host: Option<String>,

        #[command(subcommand)]
        action: ArcamAction,
    },

    /// Show shell completions setup instructions
    #[command(after_help = COMPLETIONS_HELP)]
    Completions,

    /// Sony STR-ZA/DN series receiver control
    Sony {
        /// Device name from ~/homey.json
        #[arg(long)]
        name: Option<String>,

        /// Sony receiver IP or DNS name (overrides config)
        #[arg(long, env = "SONY_RECEIVER")]
        host: Option<String>,

        /// Sony receiver port (default: 10000)
        #[arg(long, default_value = "10000")]
        port: u16,

        #[command(subcommand)]
        action: SonyAction,
    },
}

// =============================================================================
//                              ARCAM ACTIONS
// =============================================================================

#[derive(Subcommand)]
enum ArcamAction {
    /// Power on the amplifier
    On,
    /// Power off the amplifier
    Off,
    /// Get current power state
    PowerStatus,
    /// Get current mute state
    MuteStatus,
    /// Toggle mute state
    MuteToggle,
}

// =============================================================================
//                              SONY ACTIONS
// =============================================================================

#[derive(Subcommand)]
enum SonyAction {
    /// System commands (power, info, updates)
    #[command(subcommand)]
    System(SonySystemAction),

    /// Audio commands (volume, mute, sound settings)
    #[command(subcommand)]
    Audio(SonyAudioAction),

    /// Input/content commands (source selection, playback)
    #[command(subcommand)]
    Input(SonyInputAction),

    /// Playback control commands
    #[command(subcommand)]
    Playback(SonyPlaybackAction),

    /// Debug and introspection commands
    #[command(subcommand)]
    Debug(SonyDebugAction),
}

#[derive(Subcommand)]
enum SonySystemAction {
    /// Get current power status
    PowerStatus,
    /// Power on the receiver
    On,
    /// Power off the receiver
    Off,
    /// Get system information (model, serial, firmware)
    Info,
    /// Check for firmware updates
    UpdateCheck,
    /// Apply firmware update (WARNING: reboots receiver)
    UpdateApply,
    /// Get Alexa registration status
    AlexaStatus,
    /// Get ECIA device info
    EciaInfo,
    /// Get WuTang provisioning info
    WuTangInfo {
        /// Target setting name. Omit to query all.
        target: Option<String>,
    },
}

#[derive(Subcommand)]
enum SonyAudioAction {
    /// Get current volume level
    Volume,
    /// Set volume level (0-100)
    SetVolume {
        /// Volume level (0-100)
        level: u32,
    },
    /// Get mute status
    MuteStatus,
    /// Mute the receiver
    Mute,
    /// Unmute the receiver
    Unmute,
    /// Get speaker settings
    SpeakerSettings {
        /// Target setting to query
        #[arg(default_value = "all")]
        target: SpeakerTarget,
    },
}

#[derive(Subcommand)]
enum SonyInputAction {
    /// List all available inputs
    List,
    /// Get current input
    Current,
    /// Set input by URI
    Set {
        /// Input URI (e.g., extInput:hdmi?port=1)
        uri: String,
    },
    /// List available URI schemes
    Schemes,
    /// List sources for a URI scheme
    Sources {
        /// URI scheme (e.g., extInput, storage)
        scheme: String,
    },
    /// Get content count for a source
    ContentCount {
        /// Source URI (e.g., extInput:hdmi)
        source: String,
    },
    /// List content items for a source
    ContentList {
        /// Source URI (e.g., extInput:hdmi)
        source: String,
        /// Start index (default: 0)
        #[arg(long, default_value = "0")]
        start: u32,
        /// Number of items to fetch (default: 100)
        #[arg(long, default_value = "100")]
        count: u32,
    },
    /// Start content browsing for a source
    Browse {
        /// Source URI to browse
        source: String,
    },
    /// Set the active output terminal
    SetTerminal {
        /// Terminal URI
        uri: String,
    },
    /// Get Bluetooth settings
    Bluetooth {
        /// Target setting to query
        #[arg(default_value = "all")]
        target: BluetoothTarget,
    },
    /// Set a Bluetooth setting
    SetBluetooth {
        /// Target setting name
        target: String,
        /// Value to set
        value: String,
    },
    /// Get playback mode settings
    PlaybackMode {
        /// Target setting to query
        #[arg(default_value = "all")]
        target: PlaybackModeTarget,
    },
}

#[derive(Subcommand)]
enum SonyPlaybackAction {
    /// Get currently playing content info
    NowPlaying,
    /// Stop playback
    Stop,
    /// Pause playback
    Pause,
    /// Skip to next track/content
    Next,
    /// Skip to previous track/content
    Previous,
    /// Get available playback functions for current input
    Functions,
    /// Get all supported playback functions
    SupportedFunctions,
    /// Preset (save) a broadcast station
    Preset {
        /// Station URI to preset
        uri: String,
    },
    /// Seek to next/previous broadcast station
    Seek {
        /// Direction: forward or backward
        #[arg(value_enum)]
        direction: Direction,
    },
    /// Scan playing content forward/backward
    Scan {
        /// Direction: forward or backward
        #[arg(value_enum)]
        direction: Direction,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Direction {
    #[value(alias = "fwd")]
    Forward,
    #[value(alias = "bwd")]
    Backward,
}

/// Valid targets for getSpeakerSettings.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SpeakerTarget {
    /// Query all speaker settings
    All,
    /// Speaker level (dB adjustments per channel)
    Level,
    /// Speaker distance from listening position
    Distance,
    /// Speaker size (large/small)
    Size,
    /// Speaker pattern / layout
    Pattern,
}

impl SpeakerTarget {
    fn as_api_str(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Level => "level",
            Self::Distance => "distance",
            Self::Size => "size",
            Self::Pattern => "pattern",
        }
    }
}

/// Valid targets for getPlaybackModeSettings.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum PlaybackModeTarget {
    /// Query all playback mode settings
    All,
    /// Shuffle mode
    Shuffle,
    /// Repeat mode
    Repeat,
}

impl PlaybackModeTarget {
    fn as_api_str(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Shuffle => "shuffle",
            Self::Repeat => "repeat",
        }
    }
}

/// Valid targets for getBluetoothSettings.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum BluetoothTarget {
    /// Query all Bluetooth settings
    All,
    /// Bluetooth standby mode
    BtStandby,
    /// AAC codec support
    Aac,
}

impl BluetoothTarget {
    fn as_api_str(self) -> &'static str {
        match self {
            Self::All => "",
            Self::BtStandby => "btStandby",
            Self::Aac => "aac",
        }
    }
}

#[derive(Subcommand)]
enum SonyDebugAction {
    /// List supported methods for an endpoint
    Methods {
        /// API endpoint to query
        endpoint: SonyEndpoint,
    },
    /// Probe all endpoints for availability
    Probe,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SonyEndpoint {
    System,
    Audio,
    #[value(alias = "av")]
    AvContent,
    #[value(alias = "app")]
    AppControl,
    Guide,
    #[value(alias = "access")]
    AccessControl,
    Encryption,
    Browser,
}

impl From<SonyEndpoint> for SonyReceiverEndpoints {
    fn from(ep: SonyEndpoint) -> Self {
        match ep {
            SonyEndpoint::System => SonyReceiverEndpoints::System,
            SonyEndpoint::Audio => SonyReceiverEndpoints::Audio,
            SonyEndpoint::AvContent => SonyReceiverEndpoints::AvContent,
            SonyEndpoint::AppControl => SonyReceiverEndpoints::AppControl,
            SonyEndpoint::Guide => SonyReceiverEndpoints::Guide,
            SonyEndpoint::AccessControl => SonyReceiverEndpoints::AccessControl,
            SonyEndpoint::Encryption => SonyReceiverEndpoints::Encryption,
            SonyEndpoint::Browser => SonyReceiverEndpoints::Browser,
        }
    }
}

// =============================================================================
//                              MAIN
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();
    color_eyre::install()?;
    let cli = Cli::parse();

    let json = cli.json;

    match cli.command {
        Commands::Completions => {
            print!("{}", COMPLETIONS_HELP.trim_start());
            Ok(())
        }
        Commands::Arcam { action, name, host } => handle_arcam(name, host, action, json).await,
        Commands::Sony {
            action,
            name,
            host,
            port,
        } => handle_sony(name, host, port, action, json).await,
    }
}

// =============================================================================
//                              ARCAM HANDLER
// =============================================================================

async fn handle_arcam(
    name: Option<String>,
    host: Option<String>,
    action: ArcamAction,
    json: bool,
) -> Result<()> {
    let host_str = resolve_arcam_host(host, name)?;
    let arcam = Arcam::from(host_str.as_str());

    match action {
        ArcamAction::On => {
            arcam.power_on().await?;
            let is_on = arcam.request_power_state().await?;
            let status = if is_on {
                "ON"
            } else {
                "OFF (command may have been rejected)"
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": status, "verified": is_on}))?
                );
            } else {
                println!("{status}");
            }
        }
        ArcamAction::Off => {
            arcam.power_off().await?;
            let is_on = arcam.request_power_state().await?;
            let status = if !is_on {
                "OFF"
            } else {
                "ON (command may have been rejected)"
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": status, "verified": !is_on}))?
                );
            } else {
                println!("{status}");
            }
        }
        ArcamAction::PowerStatus => {
            let is_on = arcam.request_power_state().await?;
            let status = if is_on { "ON" } else { "OFF" };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": status}))?
                );
            } else {
                println!("{status}");
            }
        }
        ArcamAction::MuteStatus => {
            let is_muted = arcam.get_mute_status().await?;
            let status = if is_muted { "MUTED" } else { "UNMUTED" };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": is_muted}))?
                );
            } else {
                println!("{status}");
            }
        }
        ArcamAction::MuteToggle => {
            let was_muted = arcam.get_mute_status().await?;
            if was_muted {
                arcam.mute_off().await?;
            } else {
                arcam.mute_on().await?;
            }
            let is_muted = arcam.get_mute_status().await?;
            let status = if is_muted { "MUTED" } else { "UNMUTED" };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": is_muted}))?
                );
            } else {
                println!("{status}");
            }
        }
    }
    Ok(())
}

/// Resolves the Arcam host from --host, --name, or config auto-select.
///
/// Priority:
/// 1. `--host` (explicit override, including from `ARCAM_AMP` env)
/// 2. `--name` (config lookup by device name)
/// 3. Single device in config (auto-select when only one exists)
/// 4. Error with helpful message
fn resolve_arcam_host(host: Option<String>, name: Option<String>) -> Result<String> {
    // 1. Explicit --host flag (or ARCAM_AMP env)
    if let Some(h) = host {
        return Ok(h);
    }

    // Load config for --name and auto-select
    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.arcam_amps.get(n) {
            return Ok(format!("{}:{}", service.host, service.port));
        }
        let available = device_names(&config.arcam_amps);
        return Err(color_eyre::eyre::eyre!(
            "Arcam device '{}' not found in config.{}",
            n,
            available
        ));
    }

    // 3. Auto-select if only one device
    if config.arcam_amps.len() == 1 {
        let service = config.arcam_amps.values().next().unwrap();
        return Ok(format!("{}:{}", service.host, service.port));
    }

    // 4. Error with available devices
    let available = device_names(&config.arcam_amps);
    Err(color_eyre::eyre::eyre!(
        "Host required: use --host <IP>, --name <device>, or set ARCAM_AMP env var.{}",
        available
    ))
}

// =============================================================================
//                              SONY HANDLER
// =============================================================================

fn render_settings_table(settings: &[GenericSettingResult]) {
    let mut table = Table::new().with_columns(vec![
        TableColumn::new("Setting"),
        TableColumn::new("Value"),
        TableColumn::new("Title"),
    ]);
    for setting in settings {
        table.add_row(vec![
            setting.target.as_str().into(),
            setting.current_value.as_str().into(),
            setting.title.as_str().into(),
        ]);
    }
    print!("\n{}", table.display(&Terminal::default()));
}

async fn handle_sony(
    name: Option<String>,
    host: Option<String>,
    port: u16,
    action: SonyAction,
    json: bool,
) -> Result<()> {
    let (resolved_host, resolved_port) = resolve_sony_host(host, port, name)?;
    let receiver = SonyReceiver::new(parse_host(&resolved_host), resolved_port);

    match action {
        SonyAction::System(sys) => handle_sony_system(&receiver, sys, json).await,
        SonyAction::Audio(audio) => handle_sony_audio(&receiver, audio, json).await,
        SonyAction::Input(input) => handle_sony_input(&receiver, input, json).await,
        SonyAction::Playback(playback) => handle_sony_playback(&receiver, playback, json).await,
        SonyAction::Debug(debug) => handle_sony_debug(&receiver, debug, json).await,
    }
}

/// Resolves the Sony host/port from --host, --name, or config auto-select.
fn resolve_sony_host(
    host: Option<String>,
    port: u16,
    name: Option<String>,
) -> Result<(String, u16)> {
    // 1. Explicit --host flag (or SONY_RECEIVER env)
    if let Some(h) = host {
        let (parsed_host, parsed_port) = parse_host_port(&h, port);
        return Ok((parsed_host, parsed_port));
    }

    // Load config for --name and auto-select
    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.sony_receivers.get(n) {
            return Ok((service.host.clone(), service.port));
        }
        let available = device_names(&config.sony_receivers);
        return Err(color_eyre::eyre::eyre!(
            "Sony device '{}' not found in config.{}",
            n,
            available
        ));
    }

    // 3. Auto-select if only one device
    if config.sony_receivers.len() == 1 {
        let service = config.sony_receivers.values().next().unwrap();
        return Ok((service.host.clone(), service.port));
    }

    // 4. Error with available devices
    let available = device_names(&config.sony_receivers);
    Err(color_eyre::eyre::eyre!(
        "Host required: use --host <IP>, --name <device>, or set SONY_RECEIVER env var.{}",
        available
    ))
}

/// Parses a host string into a `Host` enum, trying IP before DNS.
fn parse_host(host: &str) -> Host {
    if let Ok(ipv4) = host.parse::<std::net::Ipv4Addr>() {
        return Host::V4(ipv4);
    }
    if let Ok(ipv6) = host.parse::<std::net::Ipv6Addr>() {
        return Host::V6(ipv6);
    }
    Host::Dns(host.to_string())
}

/// Formats available device names for error messages.
fn device_names<V>(devices: &std::collections::HashMap<String, V>) -> String {
    if devices.is_empty() {
        return String::new();
    }
    let mut names: Vec<_> = devices.keys().collect();
    names.sort();
    format!(" Available: {}", names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", "))
}

async fn handle_sony_system(
    receiver: &SonyReceiver,
    action: SonySystemAction,
    json: bool,
) -> Result<()> {
    match action {
        SonySystemAction::PowerStatus => {
            let status = receiver.get_power_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": status}))?
                );
            } else {
                println!("{status}");
            }
        }
        SonySystemAction::On => {
            receiver.set_power(true).await?;
            let status = receiver.get_power_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": status}))?
                );
            } else {
                println!("{status}");
            }
        }
        SonySystemAction::Off => {
            receiver.set_power(false).await?;
            let status = receiver.get_power_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": status}))?
                );
            } else {
                println!("{status}");
            }
        }
        SonySystemAction::Info => {
            let info = receiver.get_system_information().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec!["Model".into(), info.model.as_str().into()]);
                if let Some(serial) = &info.serial {
                    table.add_row(vec!["Serial".into(), serial.as_str().into()]);
                }
                if let Some(name) = &info.name {
                    table.add_row(vec!["Name".into(), name.as_str().into()]);
                }
                table.add_row(vec!["Firmware".into(), info.version.as_str().into()]);
                if let Some(region) = &info.region {
                    table.add_row(vec!["Region".into(), region.as_str().into()]);
                }
                table.add_row(vec!["MAC".into(), info.mac_addr.as_str().into()]);
                if let Some(wlan) = &info.wireless_mac_addr {
                    table.add_row(vec!["WLAN".into(), wlan.as_str().into()]);
                }
                if let Some(bt) = &info.bd_addr {
                    table.add_row(vec!["BT".into(), bt.as_str().into()]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        SonySystemAction::UpdateCheck => {
            let update = receiver.get_sw_update_info().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&update)?);
            } else {
                let status = if update.is_updatable == "true" {
                    "<green>Yes</green>"
                } else {
                    "<dim>No</dim>"
                };
                println!(
                    "\n{}",
                    Prose::new(format!("Updatable: {status}")).render(None)
                );
                if !update.sw_info.is_empty() {
                    let mut table = Table::new().with_columns(vec![
                        TableColumn::new("Version"),
                        TableColumn::new("Release Date"),
                        TableColumn::new("Description"),
                    ]);
                    for sw in &update.sw_info {
                        table.add_row(vec![
                            sw.version.as_str().into(),
                            sw.release_date.as_str().into(),
                            sw.description.as_deref().unwrap_or("").into(),
                        ]);
                    }
                    print!("{}", table.display(&Terminal::default()));
                }
            }
        }
        SonySystemAction::UpdateApply => {
            receiver.act_sw_update().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "update_initiated"}))?
                );
            } else {
                println!("Update initiated - receiver will reboot");
            }
        }
        SonySystemAction::AlexaStatus => match receiver.get_alexa_registration_status().await {
            Ok(status) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({"status": status}))?
                    );
                } else {
                    println!("{status}");
                }
            }
            Err(SonyError::Api(msg)) if msg.contains("No Such Method") => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &json!({"status": "not_configured", "message": "Alexa is not configured on this receiver"})
                        )?
                    );
                } else {
                    println!("Alexa is not configured on this receiver");
                    println!("Enable Alexa via the receiver's Settings > Amazon Alexa menu");
                }
            }
            Err(e) => return Err(e.into()),
        },
        SonySystemAction::EciaInfo => {
            let info = receiver.get_ecia_device_info().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("Device ID: {}", info.device_id);
            }
        }
        SonySystemAction::WuTangInfo { target } => {
            let target = target.as_deref().unwrap_or("");
            let settings = receiver.get_wu_tang_info(target).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                render_settings_table(&settings);
            }
        }
    }
    Ok(())
}

async fn handle_sony_audio(
    receiver: &SonyReceiver,
    action: SonyAudioAction,
    json: bool,
) -> Result<()> {
    match action {
        SonyAudioAction::Volume => {
            let muted = receiver.get_mute_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": muted}))?
                );
            } else {
                println!("Muted: {}", if muted { "yes" } else { "no" });
            }
        }
        SonyAudioAction::SetVolume { level } => {
            receiver.set_volume(level).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"volume": level}))?
                );
            } else {
                println!("Volume set to {level}");
            }
        }
        SonyAudioAction::MuteStatus => {
            let muted = receiver.get_mute_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": muted}))?
                );
            } else {
                println!("{}", if muted { "MUTED" } else { "UNMUTED" });
            }
        }
        SonyAudioAction::Mute => {
            receiver.set_mute(true).await?;
            let muted = receiver.get_mute_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": muted}))?
                );
            } else {
                println!("{}", if muted { "MUTED" } else { "UNMUTED (command may have been rejected)" });
            }
        }
        SonyAudioAction::Unmute => {
            receiver.set_mute(false).await?;
            let muted = receiver.get_mute_status().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": muted}))?
                );
            } else {
                println!("{}", if !muted { "UNMUTED" } else { "MUTED (command may have been rejected)" });
            }
        }
        SonyAudioAction::SpeakerSettings { target } => {
            let settings = receiver.get_speaker_settings(target.as_api_str()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                render_settings_table(&settings);
            }
        }
    }
    Ok(())
}

async fn handle_sony_input(
    receiver: &SonyReceiver,
    action: SonyInputAction,
    json: bool,
) -> Result<()> {
    match action {
        SonyInputAction::List => {
            let inputs = receiver.list_inputs().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&inputs)?);
            } else {
                let mut list = UnorderedList::empty();
                for input in inputs {
                    let connected = input.connection.as_deref() == Some("connected");
                    let status = if connected {
                        " <i>is connected</i>"
                    } else {
                        ""
                    };
                    let line = format!(
                        "{}[<gray-500>{}</gray-500>]{}",
                        input.title, input.uri, status
                    );
                    list.add(Prose::new(line));
                }
                print!("\n{}", list.display(&Terminal::default()));
            }
        }
        SonyInputAction::Current => {
            let input = receiver.get_current_input().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&input)?);
            } else {
                println!("{}", input.source.as_deref().unwrap_or(&input.uri));
            }
        }
        SonyInputAction::Set { uri } => {
            receiver.set_input(&uri).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"uri": uri}))?);
            } else {
                println!("Input set to {uri}");
            }
        }
        SonyInputAction::Schemes => {
            let schemes = receiver.get_scheme_list().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&schemes)?);
            } else {
                let mut list = UnorderedList::empty();
                for scheme in schemes {
                    list.add(Prose::new(format!("<b>{scheme}</b>")));
                }
                print!("\n{}", list.display(&Terminal::default()));
            }
        }
        SonyInputAction::Sources { scheme } => {
            let sources = receiver.get_source_list(&scheme).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&sources)?);
            } else {
                let mut list = UnorderedList::empty();
                for src in sources {
                    list.add(Prose::new(src.source));
                }
                print!("\n{}", list.display(&Terminal::default()));
            }
        }
        SonyInputAction::ContentCount { source } => {
            let count = receiver.get_content_count(&source).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"count": count}))?
                );
            } else {
                println!("{count}");
            }
        }
        SonyInputAction::ContentList {
            source,
            start,
            count,
        } => {
            let items = receiver.get_content_list(&source, start, count).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                let mut table = Table::new()
                    .with_columns(vec![TableColumn::new("Title"), TableColumn::new("URI")]);
                for item in items {
                    let title = item.title.as_deref().unwrap_or("");
                    table.add_row(vec![title.into(), item.uri.as_str().into()]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        SonyInputAction::Browse { source } => {
            receiver.start_content_browsing(&source).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "browsing", "source": source}))?
                );
            } else {
                println!("Browsing {source}");
            }
        }
        SonyInputAction::SetTerminal { uri } => {
            receiver.set_active_terminal(&uri).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"uri": uri}))?);
            } else {
                println!("Active terminal set to {uri}");
            }
        }
        SonyInputAction::Bluetooth { target } => {
            let result = receiver.get_bluetooth_settings(target.as_api_str()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if let Some(arr) = result.as_array() {
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new("Setting"),
                    TableColumn::new("Value"),
                    TableColumn::new("Title"),
                ]);
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let target = obj.get("target").and_then(|v| v.as_str()).unwrap_or("");
                        let value = obj
                            .get("currentValue")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        table.add_row(vec![target.into(), value.into(), title.into()]);
                    }
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        SonyInputAction::SetBluetooth { target, value } => {
            receiver.set_bluetooth_settings(&target, &value).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"target": target, "value": value}))?
                );
            } else {
                println!("{target} set to {value}");
            }
        }
        SonyInputAction::PlaybackMode { target } => {
            let settings = receiver
                .get_playback_mode_settings(target.as_api_str())
                .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                render_settings_table(&settings);
            }
        }
    }
    Ok(())
}

async fn handle_sony_playback(
    receiver: &SonyReceiver,
    action: SonyPlaybackAction,
    json: bool,
) -> Result<()> {
    match action {
        SonyPlaybackAction::NowPlaying => {
            let content = receiver.get_playing_content_info().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&content)?);
            } else {
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                if let Some(title) = &content.title {
                    table.add_row(vec!["Title".into(), title.as_str().into()]);
                }
                table.add_row(vec!["URI".into(), content.uri.as_str().into()]);
                if let Some(source) = &content.source {
                    table.add_row(vec!["Source".into(), source.as_str().into()]);
                }
                if let Some(state) = &content.state {
                    table.add_row(vec!["State".into(), state.as_str().into()]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        SonyPlaybackAction::Stop => {
            receiver.stop_playing_content().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "stopped"}))?
                );
            } else {
                println!("Stopped");
            }
        }
        SonyPlaybackAction::Pause => {
            receiver.pause_playing_content().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "paused"}))?
                );
            } else {
                println!("Paused");
            }
        }
        SonyPlaybackAction::Next => {
            receiver.set_play_next_content().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "next"}))?
                );
            } else {
                println!("Next");
            }
        }
        SonyPlaybackAction::Previous => {
            receiver.set_play_previous_content().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "previous"}))?
                );
            } else {
                println!("Previous");
            }
        }
        SonyPlaybackAction::Functions => {
            let funcs = receiver.get_available_playback_function().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&funcs)?);
            } else if funcs.functions.is_empty() {
                println!("none");
            } else {
                let mut list = UnorderedList::empty();
                for func in funcs.functions {
                    list.add(Prose::new(func));
                }
                print!("\n{}", list.display(&Terminal::default()));
            }
        }
        SonyPlaybackAction::SupportedFunctions => {
            let items = receiver.get_supported_playback_function().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                let mut list = UnorderedList::empty();
                for item in &items {
                    if item.functions.is_empty() {
                        list.add(Prose::new(format!(
                            "<gray-500>{}</gray-500> <dim>(none)</dim>",
                            item.uri
                        )));
                    } else {
                        let funcs = item
                            .functions
                            .iter()
                            .map(|f| f.function.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        list.add(Prose::new(format!(
                            "<gray-500>{}</gray-500>: {funcs}",
                            item.uri
                        )));
                    }
                }
                print!("\n{}", list.display(&Terminal::default()));
            }
        }
        SonyPlaybackAction::Preset { uri } => {
            receiver.preset_broadcast_station(&uri).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "preset", "uri": uri}))?
                );
            } else {
                println!("Station preset: {uri}");
            }
        }
        SonyPlaybackAction::Seek { direction } => {
            let forward = matches!(direction, Direction::Forward);
            receiver.seek_broadcast_station(forward).await?;
            let dir = if forward { "forward" } else { "backward" };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "seeking", "direction": dir}))?
                );
            } else {
                println!("Seeking {dir}");
            }
        }
        SonyPlaybackAction::Scan { direction } => {
            let forward = matches!(direction, Direction::Forward);
            receiver.scan_playing_content(forward).await?;
            let dir = if forward { "forward" } else { "backward" };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "scanning", "direction": dir}))?
                );
            } else {
                println!("Scanning {dir}");
            }
        }
    }
    Ok(())
}

async fn handle_sony_debug(
    receiver: &SonyReceiver,
    action: SonyDebugAction,
    json: bool,
) -> Result<()> {
    match action {
        SonyDebugAction::Methods { endpoint } => {
            let methods = receiver.get_supported_methods(endpoint.into()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&methods)?);
            } else {
                let mut list = UnorderedList::empty();
                for method in methods {
                    let mut parts =
                        format!("<b>{}</b> <dim>(v{})</dim>", method.name, method.version);
                    if !method.params.is_empty() {
                        let params = method.params.join(", ");
                        parts.push_str(&format!("\n  <dim>params:</dim> {params}"));
                    }
                    if !method.returns.is_empty() {
                        let returns = method.returns.join(", ");
                        parts.push_str(&format!("\n  <dim>returns:</dim> {returns}"));
                    }
                    list.add(Prose::new(parts));
                }
                print!("\n{}", list.display(&Terminal::default()));
            }
        }
        SonyDebugAction::Probe => {
            let results = receiver.probe_endpoints().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                println!(
                    "\n{}",
                    Prose::new(format!(
                        "<dim>Probing Endpoints on {}:{}</dim>",
                        receiver.host(),
                        receiver.port()
                    ))
                    .render(None)
                );
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new("Status"),
                    TableColumn::new("Path"),
                    TableColumn::new("Detail"),
                ]);
                for result in results {
                    let marker = if result.active {
                        "<green>[OK]</green>".to_string()
                    } else {
                        "<red>[ERR]</red>".to_string()
                    };
                    table.add_row(vec![
                        TableCellContent::Text(Prose::new(marker).render(None)),
                        result.path.as_str().into(),
                        result.detail.as_str().into(),
                    ]);
                }
                print!("{}", table.display(&Terminal::default()));
            }
        }
    }
    Ok(())
}
