//! Homelab automation CLI.

use biscuit_terminal::prelude::{
    Prose, Renderable, Table, TableCellContent, TableColumn, Terminal, UnorderedList,
};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::CompleteEnv;
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use homelab::arcam::{Arcam, ArcamResponse};
use homelab::config::{HomeyConfig, parse_host_port};
use homelab::network::Host;
use homelab::sony_receiver::{
    GenericSettingResult, SonyError, SonyReceiver, SonyReceiverEndpoints,
};
use serde_json::json;

// =============================================================================
//                          DEVICE RESOLUTION
// =============================================================================

/// How a device host was resolved.
enum DeviceSource {
    /// Explicit --host flag or env var
    Flag,
    /// Looked up by --name in config
    Name(String),
    /// Auto-selected as the only device in config
    Auto(String),
}

/// Renders a styled device context suffix for human-readable output.
///
/// Returns something like `(<dim>192.168.1.50:50000 <i>via</i> <blue>office</blue></dim>)`.
fn device_suffix(host: &str, port: u16, source: &DeviceSource) -> String {
    match source {
        DeviceSource::Flag => format!("(<dim>{host}:{port}</dim>)"),
        DeviceSource::Name(name) | DeviceSource::Auto(name) => {
            format!("(<dim>{host}:{port} <i>via</i> <blue>{name}</blue></dim>)")
        }
    }
}

/// Renders a styled single-line result using Prose.
fn styled(text: impl Into<String>) -> String {
    Prose::new(text).fallback_render(&Terminal::default())
}

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
    /// Probe: send power query and show raw response bytes
    Probe,
    /// Get auto shutdown setting
    AutoShutdown,
    /// Set auto shutdown (0=off, 1=20min, 2=30min, 3=1hr, 4=2hr)
    AutoShutdownSet {
        /// Auto shutdown value: 0=off, 1=20min, 2=30min, 3=1hr, 4=2hr
        value: u8,
    },
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

    /// Native Web API commands (zones, system settings, audio settings)
    #[command(subcommand)]
    Native(SonyNativeAction),

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
    /// Show input configuration (names, HDMI assignments, visibility)
    Config,
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

/// Native Web API commands (port 80)
#[derive(Subcommand)]
enum SonyNativeAction {
    /// Get main zone status (power, volume, mute, input)
    Zone,
    /// Get zone 2 status
    Zone2,
    /// Get zone 3 status
    Zone3,
    /// Get system settings (volume display, dimmer, device name, network)
    SystemSettings,
    /// Get audio settings (pure direct, sound field, speaker levels)
    AudioSettings,
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
async fn main() {
    CompleteEnv::with_factory(Cli::command).complete();
    if let Err(e) = run().await {
        // Deduplicate chain: skip causes whose message is already contained in a prior message.
        let top = e.to_string();
        let mut seen = top.clone();
        let causes: Vec<String> = e
            .chain()
            .skip(1)
            .filter_map(|c| {
                let msg = c.to_string();
                if seen.contains(&msg) {
                    None
                } else {
                    seen.push_str(&msg);
                    Some(msg)
                }
            })
            .collect();
        let msg = if causes.is_empty() {
            format!("<red><b>Error:</b></red> {top}")
        } else {
            format!(
                "<red><b>Error:</b></red> {top} <dim>▸</dim> {}",
                causes.join(" <dim>▸</dim> ")
            )
        };
        eprintln!("{}", styled(msg));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
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
    let (resolved_host, source) = resolve_arcam(host, name)?;
    let arcam = Arcam::from(resolved_host.as_str());
    let suffix = device_suffix(&resolved_host, 50000, &source);
    let err_ctx = format!("Arcam Amp at {resolved_host}:50000");

    run_arcam_action(&arcam, action, json, &suffix)
        .await
        .wrap_err(err_ctx)
}

async fn run_arcam_action(
    arcam: &Arcam,
    action: ArcamAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        // Power on/off: trust the command acknowledgment from send_command.
        // The Arcam protocol reads a response byte, so Ok(()) means the amp
        // confirmed receipt. Verification is unreliable because the amp is
        // physically transitioning and won't respond to a new TCP connection
        // until it finishes booting (on) or goes to standby (off).
        ArcamAction::On => {
            arcam.power_on().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "ON"}))?
                );
            } else {
                println!("{}", styled(format!("Sent signal to <b>Arcam Amp</b> to turn power <b>ON</b> {suffix}. Powering up will take several seconds.")));
            }
        }
        ArcamAction::Off => {
            arcam.power_off().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "OFF"}))?
                );
            } else {
                println!("{}", styled(format!("Sent signal to <b>Arcam Amp</b> to turn power <b>OFF</b> {suffix}. Powering down will take several seconds.")));
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
                println!("{}", styled(format!("<b>Arcam Amp</b> is <b>{status}</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Arcam Amp</b> is <b>{status}</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Arcam Amp</b> is now <b>{status}</b> {suffix}")));
            }
        }
        ArcamAction::Probe => {
            let format_resp = |label: &str, resp: &ArcamResponse| -> String {
                let hex: Vec<String> = resp.raw.iter().map(|b| format!("{b:02X}")).collect();
                let data_hex: Vec<String> = resp.data.iter().map(|b| format!("{b:02X}")).collect();
                format!(
                    "{label}:\n  raw:    {}\n  answer: 0x{:02X}{}\n  data:   [{}]",
                    hex.join(" "),
                    resp.answer_code,
                    if resp.answer_code == 0x00 { " (OK)" } else { " (ERROR)" },
                    data_hex.join(" "),
                )
            };

            println!("{}", styled(format!("<b>Arcam Amp</b> probe {suffix}\n")));

            // System model query
            let model_cmd = [0x21, 0x01, 0x5E, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&model_cmd).await {
                Ok(resp) => {
                    let model = String::from_utf8_lossy(&resp.data).trim().to_string();
                    println!("{}", format_resp("System model query", &resp));
                    println!("  model:  {model}");
                }
                Err(e) => println!("System model query failed: {e}"),
            }

            println!();

            // Power status query
            let power_cmd = [0x21, 0x01, 0x00, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&power_cmd).await {
                Ok(resp) => {
                    let state = if resp.data.first().copied() == Some(0x01) { "ON" } else { "OFF" };
                    println!("{}", format_resp("Power query", &resp));
                    println!("  state:  {state}");
                }
                Err(e) => println!("Power query failed: {e}"),
            }

            println!();

            // Mute status query
            let mute_cmd = [0x21, 0x01, 0x0E, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&mute_cmd).await {
                Ok(resp) => {
                    let state = if resp.data.first().copied() == Some(0x00) { "MUTED" } else { "UNMUTED" };
                    println!("{}", format_resp("Mute query", &resp));
                    println!("  state:  {state}");
                }
                Err(e) => println!("Mute query failed: {e}"),
            }

            println!();

            // Amplifier mode query
            // Firmware returns 1-indexed values (not 0-indexed as
            // documented in SH305E Issue 3): ST=1, BR=2, DM=3.
            let mode_cmd = [0x21, 0x01, 0x61, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&mode_cmd).await {
                Ok(resp) => {
                    let raw_byte = resp.data.first().copied();
                    let mode = match raw_byte {
                        Some(1) => "Stereo",
                        Some(2) => "Bridged",
                        Some(3) => "Dual Mono",
                        Some(n) => &format!("Unknown ({n})"),
                        None => "No data",
                    };
                    println!("{}", format_resp("Amp mode query", &resp));
                    println!("  mode:   {mode}");
                    if let Some(b) = raw_byte {
                        println!("  raw:    0x{b:02X}");
                    }
                }
                Err(e) => println!("Amp mode query failed: {e}"),
            }

            println!();

            // Auto shutdown query
            let auto_cmd = [0x21, 0x01, 0x58, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&auto_cmd).await {
                Ok(resp) => {
                    let value = resp.data.first().copied().unwrap_or(0);
                    let label = homelab::arcam::auto_shutdown_label(value);
                    println!("{}", format_resp("Auto shutdown query", &resp));
                    println!("  value:  {label}");
                }
                Err(e) => println!("Auto shutdown query failed: {e}"),
            }

            println!();

            // Timeout counter query
            let timeout_cmd = [0x21, 0x01, 0x55, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&timeout_cmd).await {
                Ok(resp) => {
                    let hi = resp.data.first().copied().unwrap_or(0);
                    let lo = resp.data.get(1).copied().unwrap_or(0);
                    let secs = u16::from_be_bytes([hi, lo]);
                    let mins = secs / 60;
                    let rem = secs % 60;
                    println!("{}", format_resp("Timeout counter query", &resp));
                    println!("  secs:   {secs} ({mins}m {rem}s)");
                }
                Err(e) => println!("Timeout counter query failed: {e}"),
            }
        }
        ArcamAction::AutoShutdown => {
            let auto_cmd = [0x21, 0x01, 0x58, 0x01, 0xF0, 0x0D];
            match arcam.send_command(&auto_cmd).await {
                Ok(resp) => {
                    let value = resp.data.first().copied().unwrap_or(0);
                    let label = homelab::arcam::auto_shutdown_label(value);
                    if json {
                        println!("{}", json!({ "value": value, "label": label }));
                    } else {
                        println!("Auto Shutdown: {} (0x{:02X})", label, value);
                    }
                }
                Err(e) => println!("Auto shutdown query failed: {e}"),
            }
        }
        ArcamAction::AutoShutdownSet { value } => {
            match arcam.set_auto_shutdown(value).await {
                Ok(new_value) => {
                    let label = homelab::arcam::auto_shutdown_label(new_value);
                    if json {
                        println!("{}", json!({ "value": new_value, "label": label }));
                    } else {
                        println!("Auto Shutdown set to: {} (0x{:02X})", label, new_value);
                    }
                }
                Err(e) => println!("Auto shutdown set failed: {e}"),
            }
        }
    }
    Ok(())
}

/// Resolves the Arcam host from --host, --name, or config auto-select.
///
/// Returns `(host, source)`. The host is just the hostname/IP — never includes port
/// since the Arcam protocol always uses port 50000.
///
/// Priority:
/// 1. `--host` (explicit override, including from `ARCAM_AMP` env)
/// 2. `--name` (config lookup by device name)
/// 3. Single device in config (auto-select when only one exists)
/// 4. Error with helpful message
fn resolve_arcam(host: Option<String>, name: Option<String>) -> Result<(String, DeviceSource)> {
    // 1. Explicit --host flag (or ARCAM_AMP env)
    if let Some(h) = host {
        return Ok((h, DeviceSource::Flag));
    }

    // Load config for --name and auto-select
    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.arcam_amps.get(n) {
            return Ok((service.host.clone(), DeviceSource::Name(n.clone())));
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
        let (dev_name, service) = config.arcam_amps.iter().next().unwrap();
        return Ok((
            service.host.clone(),
            DeviceSource::Auto(dev_name.clone()),
        ));
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
    let (resolved_host, resolved_port, source) = resolve_sony(host, port, name)?;
    let receiver = SonyReceiver::new(parse_host(&resolved_host), resolved_port);
    let suffix = device_suffix(&resolved_host, resolved_port, &source);
    let err_ctx = format!("Sony Receiver at {resolved_host}:{resolved_port}");

    match action {
        SonyAction::System(sys) => handle_sony_system(&receiver, sys, json, &suffix).await,
        SonyAction::Audio(audio) => handle_sony_audio(&receiver, audio, json, &suffix).await,
        SonyAction::Input(input) => handle_sony_input(&receiver, input, json, &suffix).await,
        SonyAction::Playback(pb) => handle_sony_playback(&receiver, pb, json, &suffix).await,
        SonyAction::Native(native) => handle_sony_native(&receiver, native, json, &suffix).await,
        SonyAction::Debug(debug) => handle_sony_debug(&receiver, debug, json, &suffix).await,
    }
    .wrap_err(err_ctx)
}

/// Resolves the Sony host/port from --host, --name, or config auto-select.
fn resolve_sony(
    host: Option<String>,
    port: u16,
    name: Option<String>,
) -> Result<(String, u16, DeviceSource)> {
    // 1. Explicit --host flag (or SONY_RECEIVER env)
    if let Some(h) = host {
        let (parsed_host, parsed_port) = parse_host_port(&h, port);
        return Ok((parsed_host, parsed_port, DeviceSource::Flag));
    }

    // Load config for --name and auto-select
    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.sony_receivers.get(n) {
            return Ok((service.host.clone(), service.port, DeviceSource::Name(n.clone())));
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
        let (dev_name, service) = config.sony_receivers.iter().next().unwrap();
        return Ok((
            service.host.clone(),
            service.port,
            DeviceSource::Auto(dev_name.clone()),
        ));
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
    format!(
        " Available: {}",
        names
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

async fn handle_sony_system(
    receiver: &SonyReceiver,
    action: SonySystemAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SonySystemAction::PowerStatus => {
            let status = receiver.get_power_status().await?;
            let display = match status.as_str() {
                "active" => "on",
                "standby" => "off",
                other => other,
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": display}))?
                );
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> is <b>{display}</b> {suffix}")));
            }
        }
        SonySystemAction::On => {
            send_sony_power(receiver, true, json, suffix).await?;
        }
        SonySystemAction::Off => {
            send_sony_power(receiver, false, json, suffix).await?;
        }
        SonySystemAction::Info => {
            let info = receiver.get_system_information().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> {suffix}")));
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
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonySystemAction::UpdateCheck => {
            let update = receiver.get_sw_update_info().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&update)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> {suffix}")));
                let updatable = if update.is_updatable == "true" {
                    "<green>Yes</green>"
                } else {
                    "<dim>No</dim>"
                };
                println!("{}", styled(format!("Updatable: {updatable}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> update initiated - receiver will reboot {suffix}")));
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
                    println!("{}", styled(format!("<b>Sony Receiver</b> Alexa: <b>{status}</b> {suffix}")));
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
                    println!("{}", styled(format!("<b>Sony Receiver</b> Alexa is not configured {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> Device ID: <b>{}</b> {suffix}", info.device_id)));
            }
        }
        SonySystemAction::WuTangInfo { target } => {
            let target = target.as_deref().unwrap_or("");
            let settings = receiver.get_wu_tang_info(target).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> {suffix}")));
                render_settings_table(&settings);
            }
        }
    }
    Ok(())
}

/// Send a power on/off command to the Sony receiver.
///
/// The JSON-RPC response tells us immediately whether the command succeeded.
/// A successful response means the receiver accepted the command.
async fn send_sony_power(
    receiver: &SonyReceiver,
    on: bool,
    json: bool,
    suffix: &str,
) -> Result<()> {
    let target = if on { "on" } else { "off" };
    receiver.set_power(on).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"status": target}))?
        );
    } else {
        println!(
            "{}",
            styled(format!(
                "<b>Sony Receiver</b> powered <b>{target}</b> {suffix}"
            ))
        );
    }
    Ok(())
}

async fn handle_sony_audio(
    receiver: &SonyReceiver,
    action: SonyAudioAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SonyAudioAction::Volume => {
            let info = receiver.get_volume().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                let mute_hint = if info.mute == "on" { " <dim>(muted)</dim>" } else { "" };
                println!("{}", styled(format!(
                    "<b>Sony Receiver</b> volume: <b>{}</b>{mute_hint} {suffix}",
                    info.volume
                )));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> volume set to <b>{level}</b> {suffix}")));
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
                let status = if muted { "MUTED" } else { "UNMUTED" };
                println!("{}", styled(format!("<b>Sony Receiver</b> is <b>{status}</b> {suffix}")));
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
            } else if muted {
                println!("{}", styled(format!("<b>Sony Receiver</b> is now <b>MUTED</b> {suffix}")));
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> is still <b>UNMUTED</b> <dim>(command may not have taken effect)</dim> {suffix}")));
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
            } else if !muted {
                println!("{}", styled(format!("<b>Sony Receiver</b> is now <b>UNMUTED</b> {suffix}")));
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> is still <b>MUTED</b> <dim>(command may not have taken effect)</dim> {suffix}")));
            }
        }
        SonyAudioAction::SpeakerSettings { target } => {
            let result = receiver.get_speaker_settings(target.as_api_str()).await;
            if result.is_err() && matches!(target, SpeakerTarget::All) {
                let valid = ["level", "distance", "size", "pattern"];
                let list = valid.iter().map(|v| format!("<b>{v}</b>")).collect::<Vec<_>>().join(", ");
                eprintln!(
                    "{}",
                    styled(format!(
                        "<red><b>Error:</b></red> speaker-settings requires a target. Valid targets: {list}",
                    ))
                );
                std::process::exit(1);
            }
            let settings = result?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> {suffix}")));
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
    suffix: &str,
) -> Result<()> {
    match action {
        SonyInputAction::Config => {
            let inputs = receiver.get_native_inputs().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&inputs)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> input config {suffix}"))
                );
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new("Name"),
                    TableColumn::new("HDMI"),
                    TableColumn::new("Sound Field"),
                    TableColumn::new("Visible"),
                ]);
                for input in &inputs {
                    let vis = if input.visible { "yes" } else { "no" };
                    table.add_row(vec![
                        input.name.as_str().into(),
                        input.hdmi_assign.as_str().into(),
                        input.sound_field.as_str().into(),
                        vis.into(),
                    ]);
                }
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyInputAction::List => {
            let inputs = receiver.list_inputs().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&inputs)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> inputs {suffix}")));
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
                print!("{}", list.display(&Terminal::default()));
            }
        }
        SonyInputAction::Current => {
            let input = receiver.get_current_input().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&input)?);
            } else {
                let source = input.source.as_deref().unwrap_or(&input.uri);
                println!("{}", styled(format!("<b>Sony Receiver</b> input is <b>{source}</b> {suffix}")));
            }
        }
        SonyInputAction::Set { uri } => {
            receiver.set_input(&uri).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"uri": uri}))?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> input set to <b>{uri}</b> {suffix}")));
            }
        }
        SonyInputAction::Schemes => {
            let schemes = receiver.get_scheme_list().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&schemes)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> URI schemes {suffix}")));
                let mut list = UnorderedList::empty();
                for scheme in schemes {
                    list.add(Prose::new(format!("<b>{scheme}</b>")));
                }
                print!("{}", list.display(&Terminal::default()));
            }
        }
        SonyInputAction::Sources { scheme } => {
            let sources = receiver.get_source_list(&scheme).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&sources)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> sources for <b>{scheme}</b> {suffix}")));
                let mut list = UnorderedList::empty();
                for src in sources {
                    list.add(Prose::new(src.source));
                }
                print!("{}", list.display(&Terminal::default()));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> content count for <b>{source}</b>: <b>{count}</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> content for <b>{source}</b> {suffix}")));
                let mut table = Table::new()
                    .with_columns(vec![TableColumn::new("Title"), TableColumn::new("URI")]);
                for item in items {
                    let title = item.title.as_deref().unwrap_or("");
                    table.add_row(vec![title.into(), item.uri.as_str().into()]);
                }
                print!("{}", table.display(&Terminal::default()));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> browsing <b>{source}</b> {suffix}")));
            }
        }
        SonyInputAction::SetTerminal { uri } => {
            receiver.set_active_terminal(&uri).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"uri": uri}))?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> active terminal set to <b>{uri}</b> {suffix}")));
            }
        }
        SonyInputAction::Bluetooth { target } => {
            let result = receiver.get_bluetooth_settings(target.as_api_str()).await;
            if result.is_err() && matches!(target, BluetoothTarget::All) {
                let valid = ["bt-standby", "aac"];
                let list = valid.iter().map(|v| format!("<b>{v}</b>")).collect::<Vec<_>>().join(", ");
                eprintln!(
                    "{}",
                    styled(format!(
                        "<red><b>Error:</b></red> bluetooth requires a target. Valid targets: {list}",
                    ))
                );
                std::process::exit(1);
            }
            let result = result?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> Bluetooth settings {suffix}")));
                if let Some(arr) = result.as_array() {
                    let mut table = Table::new().with_columns(vec![
                        TableColumn::new("Setting"),
                        TableColumn::new("Value"),
                        TableColumn::new("Title"),
                    ]);
                    for item in arr {
                        if let Some(obj) = item.as_object() {
                            let bt_target =
                                obj.get("target").and_then(|v| v.as_str()).unwrap_or("");
                            let value = obj
                                .get("currentValue")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let title =
                                obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
                            table.add_row(vec![bt_target.into(), value.into(), title.into()]);
                        }
                    }
                    print!("{}", table.display(&Terminal::default()));
                }
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
                println!("{}", styled(format!("<b>Sony Receiver</b> <b>{target}</b> set to <b>{value}</b> {suffix}")));
            }
        }
        SonyInputAction::PlaybackMode { target } => {
            let result = receiver
                .get_playback_mode_settings(target.as_api_str())
                .await;
            if result.is_err() && matches!(target, PlaybackModeTarget::All) {
                let valid = ["shuffle", "repeat"];
                let list = valid.iter().map(|v| format!("<b>{v}</b>")).collect::<Vec<_>>().join(", ");
                eprintln!(
                    "{}",
                    styled(format!(
                        "<red><b>Error:</b></red> playback-mode requires a target. Valid targets: {list}",
                    ))
                );
                std::process::exit(1);
            }
            let settings = result?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> playback mode {suffix}")));
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
    suffix: &str,
) -> Result<()> {
    match action {
        SonyPlaybackAction::NowPlaying => {
            let content = receiver.get_playing_content_info().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&content)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> now playing {suffix}")));
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
                print!("{}", table.display(&Terminal::default()));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> playback <b>stopped</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> playback <b>paused</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> skipped to <b>next</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> skipped to <b>previous</b> {suffix}")));
            }
        }
        SonyPlaybackAction::Functions => {
            let funcs = receiver.get_available_playback_function().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&funcs)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> available playback functions {suffix}")));
                if funcs.functions.is_empty() {
                    println!("  (none)");
                } else {
                    let mut list = UnorderedList::empty();
                    for func in funcs.functions {
                        list.add(Prose::new(func));
                    }
                    print!("{}", list.display(&Terminal::default()));
                }
            }
        }
        SonyPlaybackAction::SupportedFunctions => {
            let items = receiver.get_supported_playback_function().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> supported playback functions {suffix}")));
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
                print!("{}", list.display(&Terminal::default()));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> station preset: <b>{uri}</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> seeking <b>{dir}</b> {suffix}")));
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
                println!("{}", styled(format!("<b>Sony Receiver</b> scanning <b>{dir}</b> {suffix}")));
            }
        }
    }
    Ok(())
}

async fn handle_sony_native(
    receiver: &SonyReceiver,
    action: SonyNativeAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SonyNativeAction::Zone => {
            let status = receiver.get_main_zone_status().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> main zone {suffix}")));
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new(""),
                    TableColumn::new(""),
                ]);
                table.add_row(vec!["Power".into(), status.power.as_str().into()]);
                table.add_row(vec!["Volume".into(), status.volume.as_str().into()]);
                table.add_row(vec!["Mute".into(), status.mute.as_str().into()]);
                table.add_row(vec!["Input".into(), status.input.as_str().into()]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::Zone2 => {
            let status = receiver.get_zone2_status().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> zone 2 {suffix}")));
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new(""),
                    TableColumn::new(""),
                ]);
                table.add_row(vec!["Power".into(), status.power.as_str().into()]);
                table.add_row(vec!["Volume".into(), status.volume.as_str().into()]);
                table.add_row(vec!["Input".into(), status.input.as_str().into()]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::Zone3 => {
            let status = receiver.get_zone3_status().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> zone 3 {suffix}")));
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new(""),
                    TableColumn::new(""),
                ]);
                table.add_row(vec!["Power".into(), status.power.as_str().into()]);
                table.add_row(vec!["Volume".into(), status.volume.as_str().into()]);
                table.add_row(vec!["Input".into(), status.input.as_str().into()]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::SystemSettings => {
            let settings = receiver.get_system_settings().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> system settings {suffix}")));
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new(""),
                    TableColumn::new(""),
                ]);
                table.add_row(vec!["Volume Display".into(), settings.volume_display.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Dimmer".into(), settings.dimmer.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Device Name".into(), settings.device_name.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Wired LAN".into(), settings.network.wired.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Wireless LAN".into(), settings.network.wireless.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Internet".into(), settings.network.internet.as_deref().unwrap_or("N/A").into()]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::AudioSettings => {
            let settings = receiver.get_audio_settings().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> audio settings {suffix}")));
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new(""),
                    TableColumn::new(""),
                ]);
                table.add_row(vec!["Pure Direct".into(), settings.pure_direct.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Sound Field".into(), settings.sound_field.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Front Balance".into(), settings.front_balance.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Center Level".into(), settings.center_level.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Subwoofer Level".into(), settings.subwoofer_level.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Dolby Level".into(), settings.dolby_level.as_deref().unwrap_or("N/A").into()]);
                table.add_row(vec!["Surround Level".into(), settings.surround_level.as_deref().unwrap_or("N/A").into()]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
    }
    Ok(())
}

async fn handle_sony_debug(
    receiver: &SonyReceiver,
    action: SonyDebugAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SonyDebugAction::Methods { endpoint } => {
            let methods = receiver.get_supported_methods(endpoint.into()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&methods)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> supported methods {suffix}")));
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
                print!("{}", list.display(&Terminal::default()));
            }
        }
        SonyDebugAction::Probe => {
            let results = receiver.probe_endpoints().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                println!("{}", styled(format!("<b>Sony Receiver</b> endpoint probe {suffix}")));
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
