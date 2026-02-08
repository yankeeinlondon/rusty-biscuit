//! Homelab automation CLI.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use color_eyre::Result;
use homelab::arcam::Arcam;
use homelab::network::Host;
use homelab::sony_receiver::{SonyReceiver, SonyReceiverEndpoints};

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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Arcam PA240/PA410/PA720 amplifier control
    Arcam {
        /// Arcam host IP or DNS name
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
        /// Sony receiver IP or DNS name
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
        /// Target setting name
        target: String,
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
        /// Target: level, distance, size, pattern
        #[arg(default_value = "level")]
        target: String,
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
    /// Get playback mode settings
    PlaybackMode {
        /// Target setting
        target: String,
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
    /// Get available playback functions
    Functions,
}

#[derive(Subcommand)]
enum SonyDebugAction {
    /// List supported methods for an endpoint
    Methods {
        /// Endpoint: system, audio, avContent, appControl, guide
        endpoint: String,
    },
    /// Probe all endpoints for availability
    Probe,
}

// =============================================================================
//                              MAIN
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Completions => {
            print!("{}", COMPLETIONS_HELP.trim_start());
            Ok(())
        }
        Commands::Arcam { action, host } => handle_arcam(host, action).await,
        Commands::Sony { action, host, port } => handle_sony(host, port, action).await,
    }
}

// =============================================================================
//                              ARCAM HANDLER
// =============================================================================

async fn handle_arcam(host: Option<String>, action: ArcamAction) -> Result<()> {
    let host_str = host.ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "Host required: use --host <IP> or set ARCAM_AMP environment variable"
        )
    })?;

    let arcam = Arcam::from(host_str.as_str());

    match action {
        ArcamAction::On => {
            arcam.power_on().await?;
            println!("ON");
        }
        ArcamAction::Off => {
            arcam.power_off().await?;
            println!("OFF");
        }
        ArcamAction::PowerStatus => {
            let is_on = arcam.request_power_state().await?;
            println!("{}", if is_on { "ON" } else { "OFF" });
        }
        ArcamAction::MuteStatus => {
            let is_muted = arcam.get_mute_status().await?;
            println!("{}", if is_muted { "MUTED" } else { "UNMUTED" });
        }
        ArcamAction::MuteToggle => {
            let is_muted = arcam.get_mute_status().await?;
            if is_muted {
                arcam.mute_off().await?;
                println!("UNMUTED");
            } else {
                arcam.mute_on().await?;
                println!("MUTED");
            }
        }
    }
    Ok(())
}

// =============================================================================
//                              SONY HANDLER
// =============================================================================

async fn handle_sony(host: Option<String>, port: u16, action: SonyAction) -> Result<()> {
    let host_str = host.ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "Host required: use --host <IP> or set SONY_RECEIVER environment variable"
        )
    })?;

    let receiver = SonyReceiver::new(Host::Dns(host_str), port);

    match action {
        SonyAction::System(sys) => handle_sony_system(&receiver, sys).await,
        SonyAction::Audio(audio) => handle_sony_audio(&receiver, audio).await,
        SonyAction::Input(input) => handle_sony_input(&receiver, input).await,
        SonyAction::Playback(playback) => handle_sony_playback(&receiver, playback).await,
        SonyAction::Debug(debug) => handle_sony_debug(&receiver, debug).await,
    }
}

async fn handle_sony_system(receiver: &SonyReceiver, action: SonySystemAction) -> Result<()> {
    match action {
        SonySystemAction::PowerStatus => {
            let status = receiver.get_power_status().await?;
            println!("{status}");
        }
        SonySystemAction::On => {
            receiver.set_power(true).await?;
            println!("ON");
        }
        SonySystemAction::Off => {
            receiver.set_power(false).await?;
            println!("OFF");
        }
        SonySystemAction::Info => {
            let info = receiver.get_system_information().await?;
            println!("Model:    {}", info.model);
            println!("Serial:   {}", info.serial);
            println!("Name:     {}", info.name);
            println!("Firmware: {}", info.version);
            println!("Region:   {}", info.region);
            println!("MAC:      {}", info.mac_addr);
        }
        SonySystemAction::UpdateCheck => {
            let update = receiver.get_sw_update_info().await?;
            println!("Updatable: {}", update.is_updatable);
            for sw in &update.sw_info {
                println!("  Version: {} ({})", sw.version, sw.release_date);
                if let Some(desc) = &sw.description {
                    println!("  Description: {desc}");
                }
            }
        }
        SonySystemAction::UpdateApply => {
            receiver.act_sw_update().await?;
            println!("Update initiated - receiver will reboot");
        }
        SonySystemAction::AlexaStatus => {
            let status = receiver.get_alexa_registration_status().await?;
            println!("{status}");
        }
        SonySystemAction::EciaInfo => {
            let info = receiver.get_ecia_device_info().await?;
            println!("Device ID: {}", info.device_id);
        }
        SonySystemAction::WuTangInfo { target } => {
            let settings = receiver.get_wu_tang_info(&target).await?;
            for setting in settings {
                println!("{}: {} ({})", setting.target, setting.current_value, setting.title);
            }
        }
    }
    Ok(())
}

async fn handle_sony_audio(receiver: &SonyReceiver, action: SonyAudioAction) -> Result<()> {
    match action {
        SonyAudioAction::Volume => {
            // Get volume requires getting volume info
            let muted = receiver.get_mute_status().await?;
            println!("Muted: {}", if muted { "yes" } else { "no" });
        }
        SonyAudioAction::SetVolume { level } => {
            receiver.set_volume(level).await?;
            println!("Volume set to {level}");
        }
        SonyAudioAction::MuteStatus => {
            let muted = receiver.get_mute_status().await?;
            println!("{}", if muted { "MUTED" } else { "UNMUTED" });
        }
        SonyAudioAction::Mute => {
            receiver.set_mute(true).await?;
            println!("MUTED");
        }
        SonyAudioAction::Unmute => {
            receiver.set_mute(false).await?;
            println!("UNMUTED");
        }
        SonyAudioAction::SpeakerSettings { target } => {
            let settings = receiver.get_speaker_settings(&target).await?;
            for setting in settings {
                println!(
                    "{}: {} ({})",
                    setting.target, setting.current_value, setting.title
                );
            }
        }
    }
    Ok(())
}

async fn handle_sony_input(receiver: &SonyReceiver, action: SonyInputAction) -> Result<()> {
    match action {
        SonyInputAction::List => {
            let inputs = receiver.list_inputs().await?;
            for input in inputs {
                let active = input.active.as_deref().unwrap_or("false");
                let marker = if active == "true" { "* " } else { "  " };
                println!("{marker}{}: {}", input.title, input.uri);
            }
        }
        SonyInputAction::Current => {
            let input = receiver.get_current_input().await?;
            println!("{}: {}", input.title, input.uri);
        }
        SonyInputAction::Set { uri } => {
            receiver.set_input(&uri).await?;
            println!("Input set to {uri}");
        }
        SonyInputAction::Schemes => {
            let schemes = receiver.get_scheme_list().await?;
            for scheme in schemes {
                println!("{scheme}");
            }
        }
        SonyInputAction::PlaybackMode { target } => {
            let settings = receiver.get_playback_mode_settings(&target).await?;
            for setting in settings {
                println!(
                    "{}: {} ({})",
                    setting.target, setting.current_value, setting.title
                );
            }
        }
    }
    Ok(())
}

async fn handle_sony_playback(
    receiver: &SonyReceiver,
    action: SonyPlaybackAction,
) -> Result<()> {
    match action {
        SonyPlaybackAction::NowPlaying => {
            let content = receiver.get_playing_content_info().await?;
            println!("Title: {}", content.title);
            println!("URI:   {}", content.uri);
            if let Some(label) = content.label {
                println!("Label: {label}");
            }
        }
        SonyPlaybackAction::Stop => {
            receiver.stop_playing_content().await?;
            println!("Stopped");
        }
        SonyPlaybackAction::Pause => {
            receiver.pause_playing_content().await?;
            println!("Paused");
        }
        SonyPlaybackAction::Next => {
            receiver.set_play_next_content().await?;
            println!("Next");
        }
        SonyPlaybackAction::Functions => {
            let funcs = receiver.get_available_playback_function().await?;
            println!("Available functions:");
            for func in funcs.functions {
                println!("  - {func}");
            }
        }
    }
    Ok(())
}

async fn handle_sony_debug(receiver: &SonyReceiver, action: SonyDebugAction) -> Result<()> {
    match action {
        SonyDebugAction::Methods { endpoint } => {
            let ep = match endpoint.to_lowercase().as_str() {
                "system" => SonyReceiverEndpoints::System,
                "audio" => SonyReceiverEndpoints::Audio,
                "avcontent" | "av" => SonyReceiverEndpoints::AvContent,
                "appcontrol" | "app" => SonyReceiverEndpoints::AppControl,
                "guide" => SonyReceiverEndpoints::Guide,
                "accesscontrol" | "access" => SonyReceiverEndpoints::AccessControl,
                "encryption" => SonyReceiverEndpoints::Encryption,
                "browser" => SonyReceiverEndpoints::Browser,
                _ => {
                    return Err(color_eyre::eyre::eyre!(
                        "Unknown endpoint: {endpoint}. Valid: system, audio, avcontent, appcontrol, guide"
                    ));
                }
            };
            let methods = receiver.get_supported_methods(ep).await?;
            for method in methods {
                println!("{} (v{})", method.name, method.version);
                if !method.params.is_empty() {
                    println!("  params: {:?}", method.params);
                }
                if !method.returns.is_empty() {
                    println!("  returns: {:?}", method.returns);
                }
            }
        }
        SonyDebugAction::Probe => {
            receiver.probe_endpoints().await?;
        }
    }
    Ok(())
}
