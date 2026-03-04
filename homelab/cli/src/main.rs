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
use homelab::eversolo::{Eversolo, DEFAULT_PORT as EVERSOLO_DEFAULT_PORT};
use homelab::samsung_tv::{
    SamsungTv, DEFAULT_REST_PORT as SAMSUNG_DEFAULT_REST_PORT,
    DEFAULT_WS_PORT as SAMSUNG_DEFAULT_WS_PORT,
};
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
    Prose::new(text).render(&Terminal::default())
}

/// Formats a boolean as "on" or "off" for table display.
fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// Formats an `Option<String>` for table display, showing "N/A" when `None`.
fn opt_str(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("N/A")
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

    /// Eversolo DMP-A8 music streamer control
    Eversolo {
        /// Device name from ~/homey.json
        #[arg(long)]
        name: Option<String>,

        /// Eversolo host IP or DNS name (overrides config)
        #[arg(long, env = "EVERSOLO")]
        host: Option<String>,

        #[command(subcommand)]
        action: EversoloAction,
    },

    /// Samsung Smart TV control
    Samsung {
        /// Device name from ~/homey.json
        #[arg(long)]
        name: Option<String>,

        /// Samsung TV host IP or DNS name (overrides config)
        #[arg(long, env = "SAMSUNG_TV")]
        host: Option<String>,

        #[command(subcommand)]
        action: SamsungAction,
    },

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
//                            EVERSOLO ACTIONS
// =============================================================================

#[derive(Subcommand)]
enum EversoloAction {
    /// Device information and identity
    #[command(subcommand)]
    Device(EversoloDeviceAction),
    /// Music playback control
    #[command(subcommand)]
    Music(EversoloMusicAction),
    /// Audio routing and volume
    #[command(subcommand)]
    Audio(EversoloAudioAction),
    /// Power management
    #[command(subcommand)]
    Power(EversoloPowerAction),
    /// Display settings (screen, knob, VU meter, spectrum)
    #[command(subcommand)]
    Display(EversoloDisplayAction),
    /// Remote control key commands
    #[command(subcommand)]
    Remote(EversoloRemoteAction),
}

#[derive(Subcommand)]
enum EversoloDeviceAction {
    /// Get device model, firmware, and network information
    Info,
}

#[derive(Subcommand)]
enum EversoloMusicAction {
    /// Get current playback state and track info
    Status,
    /// Toggle play/pause
    PlayPause,
    /// Skip to next track
    Next,
    /// Go back to previous track
    Previous,
    /// Seek to position in current track
    Seek {
        /// Position in seconds
        seconds: u64,
    },
}

#[derive(Subcommand)]
enum EversoloAudioAction {
    /// Get current volume and mute state (from music status)
    Volume,
    /// Set volume level
    SetVolume {
        /// Volume level (0 to device max)
        level: u32,
    },
    /// Mute the device
    Mute,
    /// Unmute the device
    Unmute,
    /// List available audio inputs and outputs
    Routing,
    /// Set the active audio input
    SetInput {
        /// Input tag (from `routing` output)
        tag: String,
    },
    /// Set the active audio output
    SetOutput {
        /// Output tag (from `routing` output)
        tag: String,
    },
}

#[derive(Subcommand)]
enum EversoloPowerAction {
    /// List available power options
    Options,
    /// Execute a power action
    Set {
        /// Power action tag
        #[arg(value_enum)]
        action: PowerActionTag,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PowerActionTag {
    /// Power off the device
    Poweroff,
    /// Reboot the device
    Reboot,
    /// Toggle screen on/off
    Screen,
    /// Set timed shutdown
    #[value(alias = "timer")]
    Timeshutdown,
}

impl PowerActionTag {
    fn as_api_str(self) -> &'static str {
        match self {
            Self::Poweroff => "poweroff",
            Self::Reboot => "reboot",
            Self::Screen => "screen",
            Self::Timeshutdown => "timeshutdown",
        }
    }
}

#[derive(Subcommand)]
enum EversoloDisplayAction {
    /// Get current screen brightness
    ScreenBrightness,
    /// Set screen brightness level
    SetScreenBrightness {
        /// Brightness index (0 to max from screen-brightness output)
        index: u32,
    },
    /// Get current knob LED brightness
    KnobBrightness,
    /// Set knob LED brightness level
    SetKnobBrightness {
        /// Brightness index (0 to max from knob-brightness output)
        index: u32,
    },
    /// List available VU meter modes
    VuModes,
    /// Set VU meter display mode
    SetVuMode {
        /// Mode index (from `vu-modes` output)
        index: u32,
    },
    /// List available spectrum display modes
    SpectrumModes,
    /// Set spectrum display mode
    SetSpectrumMode {
        /// Mode index (from `spectrum-modes` output)
        index: u32,
    },
}

#[derive(Subcommand)]
enum EversoloRemoteAction {
    /// Send a remote control key command
    Key {
        /// Remote control key name
        key: String,
    },
    /// Send text input to the device
    Text {
        /// Text to input
        text: String,
    },
}

// =============================================================================
//                            SAMSUNG ACTIONS
// =============================================================================

#[derive(Subcommand)]
enum SamsungAction {
    /// Device information and logs
    #[command(subcommand)]
    Device(SamsungDeviceAction),
    /// App management
    #[command(subcommand)]
    App(SamsungAppAction),
    /// Remote control key commands
    #[command(subcommand)]
    Remote(SamsungRemoteAction),
}

#[derive(Subcommand)]
enum SamsungDeviceAction {
    /// Get device info (model, firmware, network)
    Info,
    /// Get server logs
    Logs,
}

#[derive(Subcommand)]
enum SamsungAppAction {
    /// Launch an application
    Launch {
        /// Launch by app ID
        #[arg(long)]
        id: Option<String>,
        /// Launch by app name
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum SamsungRemoteAction {
    /// Send a remote control key (e.g., KEY_VOLUP, KEY_POWER, KEY_HOME)
    SendKey {
        /// Key name
        key: String,
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

    /// Native Web API commands (zones, settings, IMAX, network, HDMI)
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
    /// Get audio settings (sound field, pure direct, spatial sound, Bluetooth mode)
    AudioSettings,
    /// Get IMAX Enhanced config (crossovers, upmixer, subwoofer, mode)
    ImaxConfig,
    /// Get network config (IPv4/IPv6, DNS, connection type, WiFi)
    NetworkConfig,
    /// Get HDMI config (CEC, eARC, signal formats, source assignments)
    HdmiConfig,
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
        Commands::Eversolo { action, name, host } => {
            handle_eversolo(name, host, action, json).await
        }
        Commands::Samsung { action, name, host } => {
            handle_samsung(name, host, action, json).await
        }
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
                println!(
                    "{}",
                    styled(format!(
                        "Sent signal to <b>Arcam Amp</b> to turn power <b>ON</b> {suffix}. Powering up will take several seconds."
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "Sent signal to <b>Arcam Amp</b> to turn power <b>OFF</b> {suffix}. Powering down will take several seconds."
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Arcam Amp</b> is <b>{status}</b> {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Arcam Amp</b> is <b>{status}</b> {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Arcam Amp</b> is now <b>{status}</b> {suffix}"))
                );
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
                    if resp.answer_code == 0x00 {
                        " (OK)"
                    } else {
                        " (ERROR)"
                    },
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
                    let state = if resp.data.first().copied() == Some(0x01) {
                        "ON"
                    } else {
                        "OFF"
                    };
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
                    let state = if resp.data.first().copied() == Some(0x00) {
                        "MUTED"
                    } else {
                        "UNMUTED"
                    };
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
        ArcamAction::AutoShutdownSet { value } => match arcam.set_auto_shutdown(value).await {
            Ok(new_value) => {
                let label = homelab::arcam::auto_shutdown_label(new_value);
                if json {
                    println!("{}", json!({ "value": new_value, "label": label }));
                } else {
                    println!("Auto Shutdown set to: {} (0x{:02X})", label, new_value);
                }
            }
            Err(e) => println!("Auto shutdown set failed: {e}"),
        },
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
        return Ok((service.host.clone(), DeviceSource::Auto(dev_name.clone())));
    }

    // 4. Error with available devices
    let available = device_names(&config.arcam_amps);
    Err(color_eyre::eyre::eyre!(
        "Host required: use --host <IP>, --name <device>, or set ARCAM_AMP env var.{}",
        available
    ))
}

// =============================================================================
//                            EVERSOLO HANDLER
// =============================================================================

async fn handle_eversolo(
    name: Option<String>,
    host: Option<String>,
    action: EversoloAction,
    json: bool,
) -> Result<()> {
    let (resolved_host, source) = resolve_eversolo(host, name)?;
    let (host_str, port) = parse_host_port(&resolved_host, EVERSOLO_DEFAULT_PORT);
    let eversolo = Eversolo::new(&host_str, port);
    let suffix = device_suffix(&host_str, port, &source);
    let err_ctx = format!("Eversolo at {host_str}:{port}");

    match action {
        EversoloAction::Device(a) => handle_eversolo_device(&eversolo, a, json, &suffix).await,
        EversoloAction::Music(a) => handle_eversolo_music(&eversolo, a, json, &suffix).await,
        EversoloAction::Audio(a) => handle_eversolo_audio(&eversolo, a, json, &suffix).await,
        EversoloAction::Power(a) => handle_eversolo_power(&eversolo, a, json, &suffix).await,
        EversoloAction::Display(a) => handle_eversolo_display(&eversolo, a, json, &suffix).await,
        EversoloAction::Remote(a) => handle_eversolo_remote(&eversolo, a, json, &suffix).await,
    }
    .wrap_err(err_ctx)
}

async fn handle_eversolo_device(
    eversolo: &Eversolo,
    action: EversoloDeviceAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        EversoloDeviceAction::Info => {
            let info = eversolo.get_model().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{}", styled(format!("<b>Eversolo DMP-A8</b> {suffix}")));
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec!["Model".into(), info.model.as_str().into()]);
                table.add_row(vec!["Firmware".into(), info.firmware.as_str().into()]);
                table.add_row(vec!["IP".into(), info.ip.as_str().into()]);
                table.add_row(vec!["Ethernet MAC".into(), info.net_mac.as_str().into()]);
                table.add_row(vec!["WiFi MAC".into(), info.wifi_mac.as_str().into()]);
                if let Some(ref android) = info.android_version {
                    table.add_row(vec!["Android".into(), android.as_str().into()]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
    }
    Ok(())
}

async fn handle_eversolo_music(
    eversolo: &Eversolo,
    action: EversoloMusicAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        EversoloMusicAction::Status => {
            let state = eversolo.get_state().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&state)?);
            } else {
                let status_label = match state.state {
                    0 => "stopped",
                    1 => "playing",
                    2 => "paused",
                    _ => "unknown",
                };
                println!(
                    "{}",
                    styled(format!(
                        "<b>Eversolo</b> is <b>{status_label}</b> {suffix}"
                    ))
                );

                if let Some(ref music) = state.playing_music {
                    let mut table = Table::new()
                        .with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                    if let Some(ref title) = music.title {
                        table.add_row(vec!["Title".into(), title.as_str().into()]);
                    }
                    if let Some(ref artist) = music.artist {
                        table.add_row(vec!["Artist".into(), artist.as_str().into()]);
                    }
                    if let Some(ref album) = music.album {
                        table.add_row(vec!["Album".into(), album.as_str().into()]);
                    }
                    if let Some(ref pos) = state.position {
                        let dur = state.duration.unwrap_or(0);
                        table.add_row(vec![
                            "Position".into(),
                            format!("{} / {}", format_ms(*pos), format_ms(dur))
                                .as_str()
                                .into(),
                        ]);
                    }
                    print!("\n{}", table.display(&Terminal::default()));
                }

                if let Some(ref vol) = state.volume_data {
                    println!(
                        "{}",
                        styled(format!(
                            "Volume: <b>{}</b>/{} {}",
                            vol.current_volume,
                            vol.max_volume,
                            if vol.is_mute { "(muted)" } else { "" }
                        ))
                    );
                }
            }
        }
        EversoloMusicAction::PlayPause => {
            eversolo.play_or_pause().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"action": "play_pause"}))?);
            } else {
                println!("{}", styled(format!("Toggled play/pause {suffix}")));
            }
        }
        EversoloMusicAction::Next => {
            eversolo.play_next().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"action": "next"}))?);
            } else {
                println!("{}", styled(format!("Skipped to next track {suffix}")));
            }
        }
        EversoloMusicAction::Previous => {
            eversolo.play_previous().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"action": "previous"}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Skipped to previous track {suffix}"))
                );
            }
        }
        EversoloMusicAction::Seek { seconds } => {
            let ms = seconds as i64 * 1000;
            eversolo.seek_to(ms).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"action": "seek", "seconds": seconds}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Seeked to {seconds}s {suffix}"))
                );
            }
        }
    }
    Ok(())
}

async fn handle_eversolo_audio(
    eversolo: &Eversolo,
    action: EversoloAudioAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        EversoloAudioAction::Volume => {
            let state = eversolo.get_state().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&state.volume_data)?);
            } else if let Some(ref vol) = state.volume_data {
                let mute_str = if vol.is_mute { " (muted)" } else { "" };
                let db_str = vol
                    .volume_db
                    .as_deref()
                    .map(|db| format!(" [{db}]"))
                    .unwrap_or_default();
                println!(
                    "{}",
                    styled(format!(
                        "<b>Eversolo</b> volume: <b>{}</b>/{}{db_str}{mute_str} {suffix}",
                        vol.current_volume, vol.max_volume
                    ))
                );
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Eversolo</b> volume data unavailable {suffix}"))
                );
            }
        }
        EversoloAudioAction::SetVolume { level } => {
            eversolo.set_volume(level as i64).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"volume": level}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Set volume to <b>{level}</b> {suffix}"))
                );
            }
        }
        EversoloAudioAction::Mute => {
            eversolo.set_mute(true).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": true}))?
                );
            } else {
                println!("{}", styled(format!("<b>Muted</b> {suffix}")));
            }
        }
        EversoloAudioAction::Unmute => {
            eversolo.set_mute(false).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"muted": false}))?
                );
            } else {
                println!("{}", styled(format!("<b>Unmuted</b> {suffix}")));
            }
        }
        EversoloAudioAction::Routing => {
            let io = eversolo.get_inputs_outputs().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&io)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Eversolo Audio Routing</b> {suffix}"))
                );
                let mut input_table = Table::new().with_columns(vec![
                    TableColumn::new("Input"),
                    TableColumn::new("Tag"),
                ]);
                for item in &io.input_data {
                    input_table.add_row(vec![item.name.as_str().into(), item.tag.as_str().into()]);
                }
                print!("\n{}", input_table.display(&Terminal::default()));

                let mut output_table = Table::new().with_columns(vec![
                    TableColumn::new("Output"),
                    TableColumn::new("Tag"),
                    TableColumn::new("Enabled"),
                ]);
                for item in &io.output_data {
                    output_table.add_row(vec![
                        item.name.as_str().into(),
                        item.tag.as_str().into(),
                        on_off(item.enable).into(),
                    ]);
                }
                print!("{}", output_table.display(&Terminal::default()));
            }
        }
        EversoloAudioAction::SetInput { tag } => {
            eversolo.set_input(&tag).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"input": tag}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Set input to <b>{tag}</b> {suffix}"))
                );
            }
        }
        EversoloAudioAction::SetOutput { tag } => {
            eversolo.set_output(&tag).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"output": tag}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Set output to <b>{tag}</b> {suffix}"))
                );
            }
        }
    }
    Ok(())
}

async fn handle_eversolo_power(
    eversolo: &Eversolo,
    action: EversoloPowerAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        EversoloPowerAction::Options => {
            let opts = eversolo.get_power_options().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&opts)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Eversolo Power Options</b> {suffix}"))
                );
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new("Name"),
                    TableColumn::new("Tag"),
                ]);
                for opt in &opts.data {
                    table.add_row(vec![opt.name.as_str().into(), opt.tag.as_str().into()]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        EversoloPowerAction::Set { action: tag } => {
            let tag_str = tag.as_api_str();
            eversolo.set_power_option(tag_str).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"power_action": tag_str}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Sent power action <b>{tag_str}</b> {suffix}"))
                );
            }
        }
    }
    Ok(())
}

async fn handle_eversolo_display(
    eversolo: &Eversolo,
    action: EversoloDisplayAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        EversoloDisplayAction::ScreenBrightness => {
            let resp = eversolo.get_screen_brightness().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                let max_str = resp
                    .max
                    .map(|m| format!("/{m}"))
                    .unwrap_or_default();
                println!(
                    "{}",
                    styled(format!(
                        "Screen brightness: <b>{}</b>{max_str} {suffix}",
                        resp.current_value
                    ))
                );
            }
        }
        EversoloDisplayAction::SetScreenBrightness { index } => {
            eversolo.set_screen_brightness(index as i64).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"screen_brightness": index}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "Set screen brightness to <b>{index}</b> {suffix}"
                    ))
                );
            }
        }
        EversoloDisplayAction::KnobBrightness => {
            let resp = eversolo.get_knob_brightness().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                let max_str = resp
                    .max
                    .map(|m| format!("/{m}"))
                    .unwrap_or_default();
                println!(
                    "{}",
                    styled(format!(
                        "Knob brightness: <b>{}</b>{max_str} {suffix}",
                        resp.current_value
                    ))
                );
            }
        }
        EversoloDisplayAction::SetKnobBrightness { index } => {
            eversolo.set_knob_brightness(index as i64).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"knob_brightness": index}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "Set knob brightness to <b>{index}</b> {suffix}"
                    ))
                );
            }
        }
        EversoloDisplayAction::VuModes => {
            let resp = eversolo.get_vu_modes().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>VU Meter Modes</b> {suffix}"))
                );
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new("Index"),
                    TableColumn::new("Name"),
                ]);
                for (i, mode) in resp.data.iter().enumerate() {
                    let idx = mode
                        .index
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| i.to_string());
                    table.add_row(vec![
                        idx.as_str().into(),
                        mode.title.as_str().into(),
                    ]);
                }
                if let Some(current) = resp.current_index {
                    table.add_row(vec![
                        "Current".into(),
                        current.to_string().as_str().into(),
                    ]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        EversoloDisplayAction::SetVuMode { index } => {
            eversolo.set_vu_mode(index as i64).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"vu_mode": index}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Set VU mode to <b>{index}</b> {suffix}"))
                );
            }
        }
        EversoloDisplayAction::SpectrumModes => {
            let resp = eversolo.get_spectrum_modes().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Spectrum Modes</b> {suffix}"))
                );
                let mut table = Table::new().with_columns(vec![
                    TableColumn::new("Index"),
                    TableColumn::new("Name"),
                ]);
                for (i, mode) in resp.data.iter().enumerate() {
                    let idx = mode
                        .index
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| i.to_string());
                    table.add_row(vec![
                        idx.as_str().into(),
                        mode.title.as_str().into(),
                    ]);
                }
                if let Some(current) = resp.current_index {
                    table.add_row(vec![
                        "Current".into(),
                        current.to_string().as_str().into(),
                    ]);
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        EversoloDisplayAction::SetSpectrumMode { index } => {
            eversolo.set_spectrum_mode(index as i64).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"spectrum_mode": index}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "Set spectrum mode to <b>{index}</b> {suffix}"
                    ))
                );
            }
        }
    }
    Ok(())
}

async fn handle_eversolo_remote(
    eversolo: &Eversolo,
    action: EversoloRemoteAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        EversoloRemoteAction::Key { key } => {
            eversolo.send_key(&key).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"key": key}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Sent key <b>{key}</b> {suffix}"))
                );
            }
        }
        EversoloRemoteAction::Text { text } => {
            eversolo.input_text(&text).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"text": text}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Sent text input {suffix}"))
                );
            }
        }
    }
    Ok(())
}

/// Formats milliseconds as `mm:ss` or `h:mm:ss`.
fn format_ms(ms: i64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Resolves the Eversolo host from --host, --name, or config auto-select.
fn resolve_eversolo(
    host: Option<String>,
    name: Option<String>,
) -> Result<(String, DeviceSource)> {
    // 1. Explicit --host flag (or EVERSOLO env)
    if let Some(h) = host {
        return Ok((h, DeviceSource::Flag));
    }

    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.eversolo_devices.get(n) {
            return Ok((service.host.clone(), DeviceSource::Name(n.clone())));
        }
        let available = device_names(&config.eversolo_devices);
        return Err(color_eyre::eyre::eyre!(
            "Eversolo device '{}' not found in config.{}",
            n,
            available
        ));
    }

    // 3. Auto-select if only one device
    if config.eversolo_devices.len() == 1 {
        let (dev_name, service) = config.eversolo_devices.iter().next().unwrap();
        return Ok((service.host.clone(), DeviceSource::Auto(dev_name.clone())));
    }

    // 4. Error with available devices
    let available = device_names(&config.eversolo_devices);
    Err(color_eyre::eyre::eyre!(
        "Host required: use --host <IP>, --name <device>, or set EVERSOLO env var.{}",
        available
    ))
}

// =============================================================================
//                            SAMSUNG HANDLER
// =============================================================================

async fn handle_samsung(
    name: Option<String>,
    host: Option<String>,
    action: SamsungAction,
    json: bool,
) -> Result<()> {
    let (resolved_host, source) = resolve_samsung(host, name)?;
    let (host_str, rest_port) =
        parse_host_port(&resolved_host, SAMSUNG_DEFAULT_REST_PORT);
    let tv = SamsungTv::new(&host_str, rest_port, SAMSUNG_DEFAULT_WS_PORT);
    let suffix = device_suffix(&host_str, rest_port, &source);
    let err_ctx = format!("Samsung TV at {host_str}:{rest_port}");

    match action {
        SamsungAction::Device(a) => handle_samsung_device(&tv, a, json, &suffix).await,
        SamsungAction::App(a) => handle_samsung_app(&tv, a, json, &suffix).await,
        SamsungAction::Remote(a) => handle_samsung_remote(&tv, a, json, &suffix).await,
    }
    .wrap_err(err_ctx)
}

async fn handle_samsung_device(
    tv: &SamsungTv,
    action: SamsungDeviceAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SamsungDeviceAction::Info => {
            let info = tv.get_device_info().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                let name = info.name.as_deref().unwrap_or("Samsung TV");
                println!("{}", styled(format!("<b>{name}</b> {suffix}")));
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                if let Some(ref id) = info.id {
                    table.add_row(vec!["ID".into(), id.as_str().into()]);
                }
                if let Some(ref device) = info.device {
                    if let Some(ref model) = device.model {
                        table.add_row(vec!["Model".into(), model.as_str().into()]);
                    }
                    if let Some(ref model_name) = device.model_name {
                        table.add_row(vec!["Model Name".into(), model_name.as_str().into()]);
                    }
                    if let Some(ref os) = device.os {
                        table.add_row(vec!["OS".into(), os.as_str().into()]);
                    }
                    if let Some(ref resolution) = device.resolution {
                        table.add_row(vec!["Resolution".into(), resolution.as_str().into()]);
                    }
                    if let Some(ref net_type) = device.network_type {
                        table.add_row(vec!["Network".into(), net_type.as_str().into()]);
                    }
                }
                print!("\n{}", table.display(&Terminal::default()));
            }
        }
        SamsungDeviceAction::Logs => {
            let logs = tv.get_server_logs().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"logs": logs}))?);
            } else {
                println!("{}", styled(format!("<b>Server Logs</b> {suffix}")));
                println!("{logs}");
            }
        }
    }
    Ok(())
}

async fn handle_samsung_app(
    tv: &SamsungTv,
    action: SamsungAppAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SamsungAppAction::Launch { id, name } => {
            match (id, name) {
                (Some(app_id), _) => {
                    tv.launch_app_by_id(&app_id).await?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({"launched_by": "id", "app_id": app_id}))?
                        );
                    } else {
                        println!(
                            "{}",
                            styled(format!("Launched app <b>{app_id}</b> {suffix}"))
                        );
                    }
                }
                (None, Some(app_name)) => {
                    tv.launch_app_by_name(&app_name).await?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({"launched_by": "name", "app_name": app_name}))?
                        );
                    } else {
                        println!(
                            "{}",
                            styled(format!("Launched app <b>{app_name}</b> {suffix}"))
                        );
                    }
                }
                (None, None) => {
                    return Err(color_eyre::eyre::eyre!(
                        "Either --id or --name is required for app launch"
                    ));
                }
            }
        }
    }
    Ok(())
}

async fn handle_samsung_remote(
    tv: &SamsungTv,
    action: SamsungRemoteAction,
    json: bool,
    suffix: &str,
) -> Result<()> {
    match action {
        SamsungRemoteAction::SendKey { key } => {
            tv.send_key(&key).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"key": key}))?
                );
            } else {
                println!(
                    "{}",
                    styled(format!("Sent key <b>{key}</b> {suffix}"))
                );
            }
        }
    }
    Ok(())
}

/// Resolves the Samsung TV host from --host, --name, or config auto-select.
fn resolve_samsung(
    host: Option<String>,
    name: Option<String>,
) -> Result<(String, DeviceSource)> {
    // 1. Explicit --host flag (or SAMSUNG_TV env)
    if let Some(h) = host {
        return Ok((h, DeviceSource::Flag));
    }

    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.samsung_tvs.get(n) {
            return Ok((service.host.clone(), DeviceSource::Name(n.clone())));
        }
        let available = device_names(&config.samsung_tvs);
        return Err(color_eyre::eyre::eyre!(
            "Samsung TV '{}' not found in config.{}",
            n,
            available
        ));
    }

    // 3. Auto-select if only one device
    if config.samsung_tvs.len() == 1 {
        let (dev_name, service) = config.samsung_tvs.iter().next().unwrap();
        return Ok((service.host.clone(), DeviceSource::Auto(dev_name.clone())));
    }

    // 4. Error with available devices
    let available = device_names(&config.samsung_tvs);
    Err(color_eyre::eyre::eyre!(
        "Host required: use --host <IP>, --name <device>, or set SAMSUNG_TV env var.{}",
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
            return Ok((
                service.host.clone(),
                service.port,
                DeviceSource::Name(n.clone()),
            ));
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> is <b>{display}</b> {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> update initiated - receiver will reboot {suffix}"
                    ))
                );
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
                    println!(
                        "{}",
                        styled(format!(
                            "<b>Sony Receiver</b> Alexa: <b>{status}</b> {suffix}"
                        ))
                    );
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
                    println!(
                        "{}",
                        styled(format!(
                            "<b>Sony Receiver</b> Alexa is not configured {suffix}"
                        ))
                    );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> Device ID: <b>{}</b> {suffix}",
                        info.device_id
                    ))
                );
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
                let mute_hint = if info.mute == "on" {
                    " <dim>(muted)</dim>"
                } else {
                    ""
                };
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> volume: <b>{}</b>{mute_hint} {suffix}",
                        info.volume
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> volume set to <b>{level}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> is <b>{status}</b> {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> is now <b>MUTED</b> {suffix}"))
                );
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> is still <b>UNMUTED</b> <dim>(command may not have taken effect)</dim> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> is now <b>UNMUTED</b> {suffix}"
                    ))
                );
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> is still <b>MUTED</b> <dim>(command may not have taken effect)</dim> {suffix}"
                    ))
                );
            }
        }
        SonyAudioAction::SpeakerSettings { target } => {
            let result = receiver.get_speaker_settings(target.as_api_str()).await;
            if result.is_err() && matches!(target, SpeakerTarget::All) {
                let valid = ["level", "distance", "size", "pattern"];
                let list = valid
                    .iter()
                    .map(|v| format!("<b>{v}</b>"))
                    .collect::<Vec<_>>()
                    .join(", ");
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> inputs {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> input is <b>{source}</b> {suffix}"
                    ))
                );
            }
        }
        SonyInputAction::Set { uri } => {
            receiver.set_input(&uri).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"uri": uri}))?);
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> input set to <b>{uri}</b> {suffix}"
                    ))
                );
            }
        }
        SonyInputAction::Schemes => {
            let schemes = receiver.get_scheme_list().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&schemes)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> URI schemes {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> sources for <b>{scheme}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> content count for <b>{source}</b>: <b>{count}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> content for <b>{source}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> browsing <b>{source}</b> {suffix}"
                    ))
                );
            }
        }
        SonyInputAction::SetTerminal { uri } => {
            receiver.set_active_terminal(&uri).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&json!({"uri": uri}))?);
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> active terminal set to <b>{uri}</b> {suffix}"
                    ))
                );
            }
        }
        SonyInputAction::Bluetooth { target } => {
            let result = receiver.get_bluetooth_settings(target.as_api_str()).await;
            if result.is_err() && matches!(target, BluetoothTarget::All) {
                let valid = ["bt-standby", "aac"];
                let list = valid
                    .iter()
                    .map(|v| format!("<b>{v}</b>"))
                    .collect::<Vec<_>>()
                    .join(", ");
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> Bluetooth settings {suffix}"))
                );
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
                            let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> <b>{target}</b> set to <b>{value}</b> {suffix}"
                    ))
                );
            }
        }
        SonyInputAction::PlaybackMode { target } => {
            let result = receiver
                .get_playback_mode_settings(target.as_api_str())
                .await;
            if result.is_err() && matches!(target, PlaybackModeTarget::All) {
                let valid = ["shuffle", "repeat"];
                let list = valid
                    .iter()
                    .map(|v| format!("<b>{v}</b>"))
                    .collect::<Vec<_>>()
                    .join(", ");
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> playback mode {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> now playing {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> playback <b>stopped</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> playback <b>paused</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> skipped to <b>next</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> skipped to <b>previous</b> {suffix}"
                    ))
                );
            }
        }
        SonyPlaybackAction::Functions => {
            let funcs = receiver.get_available_playback_function().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&funcs)?);
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> available playback functions {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> supported playback functions {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> station preset: <b>{uri}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> seeking <b>{dir}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> scanning <b>{dir}</b> {suffix}"
                    ))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> main zone {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> zone 2 {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> zone 3 {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> system settings {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec![
                    "Volume Display".into(),
                    settings.volume_display.as_deref().unwrap_or("N/A").into(),
                ]);
                table.add_row(vec![
                    "Dimmer".into(),
                    settings.dimmer.as_deref().unwrap_or("N/A").into(),
                ]);
                table.add_row(vec![
                    "Device Name".into(),
                    settings.device_name.as_deref().unwrap_or("N/A").into(),
                ]);
                table.add_row(vec![
                    "Wired LAN".into(),
                    settings.network.wired.as_deref().unwrap_or("N/A").into(),
                ]);
                table.add_row(vec![
                    "Wireless LAN".into(),
                    settings.network.wireless.as_deref().unwrap_or("N/A").into(),
                ]);
                table.add_row(vec![
                    "Internet".into(),
                    settings.network.internet.as_deref().unwrap_or("N/A").into(),
                ]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::AudioSettings => {
            let settings = receiver.get_audio_settings().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> audio settings {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec![
                    "Sound Field".into(),
                    settings.sound_field.as_str().into(),
                ]);
                table.add_row(vec![
                    "Pure Direct".into(),
                    on_off(settings.pure_direct).into(),
                ]);
                table.add_row(vec![
                    "Headphones".into(),
                    on_off(settings.headphones_inserted).into(),
                ]);
                table.add_row(vec![
                    "360 Spatial Sound".into(),
                    on_off(settings.spatial_sound_360).into(),
                ]);
                table.add_row(vec![
                    "Speaker Relocation".into(),
                    on_off(settings.speaker_relocation).into(),
                ]);
                table.add_row(vec![
                    "DSD Native".into(),
                    on_off(settings.dsd_native).into(),
                ]);
                table.add_row(vec![
                    "Subwoofer LPF".into(),
                    on_off(settings.subwoofer_lpf).into(),
                ]);
                table.add_row(vec!["A/V Sync".into(), settings.av_sync.as_str().into()]);
                table.add_row(vec!["Dual Mono".into(), settings.dual_mono.as_str().into()]);
                table.add_row(vec![
                    "DRC".into(),
                    on_off(settings.dynamic_range_compression).into(),
                ]);
                table.add_row(vec![
                    "Bluetooth Mode".into(),
                    settings.bluetooth_mode.as_str().into(),
                ]);
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::ImaxConfig => {
            let config = receiver.get_imax_config().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!(
                    "{}",
                    styled(format!(
                        "<b>Sony Receiver</b> IMAX Enhanced config {suffix}"
                    ))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec!["Mode".into(), config.mode.as_str().into()]);
                table.add_row(vec!["Upmixer".into(), config.upmixer.as_str().into()]);
                table.add_row(vec![
                    "Virtualizer".into(),
                    on_off(config.virtualizer).into(),
                ]);
                table.add_row(vec![
                    "Subwoofer LPF".into(),
                    opt_str(&config.lpf_subwoofer).into(),
                ]);
                table.add_row(vec![
                    "Subwoofer Volume".into(),
                    opt_str(&config.subwoofer_volume).into(),
                ]);
                table.add_row(vec![
                    "Subwoofer Redirect".into(),
                    on_off(config.subwoofer_redirect).into(),
                ]);

                let active: Vec<_> = config
                    .crossovers
                    .iter()
                    .filter(|c| c.value.is_some())
                    .collect();
                if !active.is_empty() {
                    println!();
                    println!("{}", styled("<b>HPF Crossovers</b>".to_string()));
                    let mut cross_table = Table::new().with_columns(vec![
                        TableColumn::new("Position"),
                        TableColumn::new("Frequency"),
                    ]);
                    for c in &active {
                        cross_table.add_row(vec![
                            c.position.as_str().into(),
                            c.value.as_deref().unwrap_or("N/A").into(),
                        ]);
                    }
                    print!("{}", cross_table.display(&Terminal::default()));
                }
            }
        }
        SonyNativeAction::NetworkConfig => {
            let config = receiver.get_network_config().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> network config {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec![
                    "Connection".into(),
                    config.connection_type.as_str().into(),
                ]);
                table.add_row(vec!["IPv4 DHCP".into(), on_off(config.ipv4_dhcp).into()]);
                table.add_row(vec![
                    "IPv4 Address".into(),
                    config.ipv4_address.as_str().into(),
                ]);
                table.add_row(vec![
                    "Subnet Mask".into(),
                    config.ipv4_subnet.as_str().into(),
                ]);
                table.add_row(vec!["Gateway".into(), config.ipv4_gateway.as_str().into()]);
                table.add_row(vec!["DNS 1".into(), config.dns1.as_str().into()]);
                table.add_row(vec!["DNS 2".into(), opt_str(&config.dns2).into()]);
                table.add_row(vec!["IPv6".into(), on_off(config.ipv6_enabled).into()]);
                if let Some(ref ssid) = config.wifi_ssid {
                    table.add_row(vec!["WiFi SSID".into(), ssid.as_str().into()]);
                }
                if let Some(ref auth) = config.wifi_auth {
                    table.add_row(vec!["WiFi Auth".into(), auth.as_str().into()]);
                }
                print!("{}", table.display(&Terminal::default()));
            }
        }
        SonyNativeAction::HdmiConfig => {
            let config = receiver.get_hdmi_config().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> HDMI config {suffix}"))
                );
                let mut table =
                    Table::new().with_columns(vec![TableColumn::new(""), TableColumn::new("")]);
                table.add_row(vec![
                    "4K/8K Scaling".into(),
                    config.scaling_4k8k.as_str().into(),
                ]);
                table.add_row(vec!["CEC".into(), on_off(config.cec).into()]);
                table.add_row(vec![
                    "Standby Link".into(),
                    config.standby_link.as_str().into(),
                ]);
                table.add_row(vec![
                    "Passthrough".into(),
                    config.passthrough.as_str().into(),
                ]);
                table.add_row(vec![
                    "Audio Return".into(),
                    config.audio_return_channel.as_str().into(),
                ]);
                table.add_row(vec!["Audio Out".into(), config.audio_out.as_str().into()]);
                table.add_row(vec![
                    "Zone 2 Audio Out".into(),
                    config.zone2_audio_out.as_str().into(),
                ]);
                table.add_row(vec![
                    "Subwoofer Level".into(),
                    config.subwoofer_level.as_str().into(),
                ]);
                table.add_row(vec!["Output 2".into(), config.out2.as_str().into()]);
                table.add_row(vec!["Fast View".into(), on_off(config.fast_view).into()]);
                table.add_row(vec!["Output 4 PIP".into(), on_off(config.out4_pip).into()]);

                println!();
                println!("{}", styled("<b>Output Capabilities</b>".to_string()));
                let mut out_table = Table::new().with_columns(vec![
                    TableColumn::new(""),
                    TableColumn::new("Output A"),
                    TableColumn::new("Output B"),
                ]);
                out_table.add_row(vec![
                    "Video Format".into(),
                    config.video_format_a.as_str().into(),
                    config.video_format_b.as_str().into(),
                ]);
                out_table.add_row(vec![
                    "HDR Format".into(),
                    config.hdr_format_a.as_str().into(),
                    config.hdr_format_b.as_str().into(),
                ]);
                out_table.add_row(vec![
                    "Other".into(),
                    config.other_features_a.as_str().into(),
                    config.other_features_b.as_str().into(),
                ]);
                print!("{}", out_table.display(&Terminal::default()));

                println!();
                println!("{}", styled("<b>Port Signal Formats</b>".to_string()));
                let mut port_table = Table::new().with_columns(vec![
                    TableColumn::new("Port"),
                    TableColumn::new("Signal Format"),
                ]);
                for p in &config.port_signal_formats {
                    port_table.add_row(vec![
                        format!("HDMI {}", p.port).into(),
                        p.signal_format.as_str().into(),
                    ]);
                }
                print!("{}", port_table.display(&Terminal::default()));

                println!();
                println!("{}", styled("<b>Source Assignments</b>".to_string()));
                let mut src_table = Table::new().with_columns(vec![
                    TableColumn::new("Source"),
                    TableColumn::new("Signal Format"),
                ]);
                for s in &config.source_assignments {
                    src_table.add_row(vec![
                        s.source.as_str().into(),
                        s.signal_format.as_str().into(),
                    ]);
                }
                print!("{}", src_table.display(&Terminal::default()));
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> supported methods {suffix}"))
                );
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
                println!(
                    "{}",
                    styled(format!("<b>Sony Receiver</b> endpoint probe {suffix}"))
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
                        TableCellContent::Text(Prose::new(marker).render_optimistic(None)),
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
