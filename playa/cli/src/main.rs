use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use clap::builder::PossibleValue;
use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::CompleteEnv;
use sniff::programs::InstalledHeadlessAudio;

use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use playa::{AudioFileFormat, AudioPlayer, Codec, PLAYER_LOOKUP, Playa, SoundEffect, all_players};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};
use darkmatter::testing::strip_ansi_codes;
#[cfg(feature = "audio-ducking")]
use playa::ducking::{DuckConfig, backend_name, create_backend};

const MISSING_FG: &str = "\x1b[38;2;140;140;140m";
const RESET: &str = "\x1b[0m";
const TABLE_DIVIDER: char = '\u{2502}';

/// Print a styled error to stderr and exit.
fn error_exit(message: &str, code: i32) -> ! {
    let styled = format!("<b>Playa(<red-500>error</red-500>):</b> {message}");
    let rendered = Prose::new(styled).render(&Terminal::default());
    eprintln!("{rendered}");
    std::process::exit(code)
}

const AFTER_HELP: &str = "\
Shell Completions:
  Enable completions by adding one of the following to your shell config:

  # Bash (~/.bashrc)
  source <(COMPLETE=bash playa)

  # Zsh (~/.zshrc)
  source <(COMPLETE=zsh playa)

  # Fish (~/.config/fish/config.fish)
  COMPLETE=fish playa | source";

/// Play audio using the host's installed players
#[derive(Parser)]
#[command(name = "playa")]
#[command(about = "Play audio using the host's installed players", long_about = None)]
#[command(after_help = AFTER_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Audio file to play (shorthand for `playa play <file>`)
    #[arg(value_name = "AUDIO_FILE", value_hint = ValueHint::FilePath)]
    audio_file: Option<PathBuf>,

    #[command(flatten)]
    playback: PlaybackOptions,
}

#[derive(Subcommand)]
enum Command {
    /// Play an audio file
    Play {
        /// Audio file to play
        #[arg(value_name = "AUDIO_FILE", value_hint = ValueHint::FilePath)]
        audio_file: PathBuf,

        #[command(flatten)]
        playback: PlaybackOptions,
    },

    /// Play a built-in sound effect
    Effect {
        /// Name of the sound effect to play
        #[arg(value_name = "NAME", value_parser = EffectNameParser)]
        name: String,

        #[command(flatten)]
        playback: PlaybackOptions,
    },

    /// List available built-in sound effects
    ListEffects {
        /// Filter effects by name, description, or category (case-insensitive)
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },

    /// Show a table of available audio players
    Players,

    /// Show available native output channels (audio devices)
    #[cfg(feature = "sfx-native")]
    OutputChannels,

    /// Show audio ducking backend info
    #[cfg(feature = "audio-ducking")]
    DuckInfo,
}

/// Value parser that provides sound effect names for shell completion
/// while accepting any string (preserving fuzzy matching in the handler).
#[derive(Clone)]
struct EffectNameParser;

impl clap::builder::TypedValueParser for EffectNameParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .map(String::from)
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        let values = SoundEffect::all()
            .into_iter()
            .map(|e| PossibleValue::new(e.name()).help(e.description()));
        Some(Box::new(values))
    }
}

/// Value parser that suggests common volume levels for shell completion
/// while accepting any valid f32 in range.
#[derive(Clone)]
struct VolumeParser;

impl clap::builder::TypedValueParser for VolumeParser {
    type Value = f32;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value
            .to_str()
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))?;
        s.parse::<f32>()
            .map_err(|_| clap::Error::new(clap::error::ErrorKind::InvalidValue))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        let values = [
            ("0.25", "25% volume"),
            ("0.5", "50% volume"),
            ("0.75", "75% volume"),
            ("1", "100% volume"),
            ("1.25", "125% volume"),
            ("1.5", "150% volume"),
            ("2", "200% volume"),
        ]
        .into_iter()
        .map(|(val, help)| PossibleValue::new(val).help(help));
        Some(Box::new(values))
    }
}

/// Value parser that suggests available native output channels for shell completion.
#[derive(Clone)]
struct ChannelParser;

impl clap::builder::TypedValueParser for ChannelParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .map(String::from)
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        #[cfg(feature = "sfx-native")]
        {
            if let Ok(channels) = playa::get_output_channels() {
                let values = channels.into_iter().map(|c| {
                    let mut help = c.name;
                    if c.is_default_audio {
                        help.push_str(" (default audio)");
                    }
                    if c.is_default_sfx {
                        help.push_str(" (default sfx)");
                    }
                    let static_name: &'static str = Box::leak(c.id.into_boxed_str());
                    let static_help: &'static str = Box::leak(help.into_boxed_str());
                    PossibleValue::new(static_name).help(static_help)
                });
                return Some(Box::new(values));
            }
        }
        None
    }
}

/// Playback options shared between play and effect commands
#[derive(Parser, Clone)]
struct PlaybackOptions {
    /// Display playback metadata (player, volume, speed, codec, format)
    #[arg(long)]
    meta: bool,

    /// Play in the background and return control to the terminal immediately
    #[arg(long)]
    background: bool,

    /// Play at 1.25x speed
    #[arg(long, conflicts_with = "slow")]
    fast: bool,

    /// Play at 0.75x speed
    #[arg(long, conflicts_with = "fast")]
    slow: bool,

    /// Play at 50% volume
    #[arg(long, conflicts_with = "loud")]
    quiet: bool,

    /// Play at 150% volume
    #[arg(long, conflicts_with = "quiet")]
    loud: bool,

    /// Custom playback speed (0.5 to 2.0)
    #[arg(long, value_name = "MULTIPLIER", conflicts_with_all = ["fast", "slow"])]
    speed: Option<f32>,

    /// Custom volume level (0.0 to 2.0)
    #[arg(long, value_name = "LEVEL", conflicts_with_all = ["quiet", "loud"], value_parser = VolumeParser)]
    volume: Option<f32>,

    /// Specific output channel to use for playback, by name
    #[arg(long, value_name = "CHANNEL", value_parser = ChannelParser)]
    channel: Option<String>,

    /// Force host player playback (skip native decoder)
    #[arg(long)]
    force_host: bool,

    /// Disable audio ducking (attenuating other audio during playback)
    #[cfg(feature = "audio-ducking")]
    #[arg(long)]
    no_duck: bool,

    /// Ducking ramp duration in milliseconds (default: 1000)
    #[cfg(feature = "audio-ducking")]
    #[arg(long, value_name = "MS", default_value = "1000")]
    duck_ramp_ms: u32,

    /// Ducking floor level (0.0 = silent, 1.0 = no ducking, default: 0.2)
    #[cfg(feature = "audio-ducking")]
    #[arg(long, value_name = "LEVEL", default_value = "0.2")]
    duck_floor: f32,
}

impl PlaybackOptions {
    /// Convert CLI playback options to the library's `PlaybackOptions` type.
    fn to_lib_options(&self) -> playa::PlaybackOptions {
        let mut opts = playa::PlaybackOptions::new();

        if let Some(speed) = self.speed {
            opts = opts.with_speed(speed);
        } else if self.fast {
            opts = opts.with_speed(1.25);
        } else if self.slow {
            opts = opts.with_speed(0.75);
        }

        if let Some(volume) = self.volume {
            opts = opts.with_volume(volume);
        } else if self.quiet {
            opts = opts.with_volume(0.5);
        } else if self.loud {
            opts = opts.with_volume(1.5);
        }

        if let Some(channel) = &self.channel {
            opts = opts.with_channel(channel.clone());
        }

        opts
    }

    fn apply_to_playa(&self, mut playa: Playa) -> Playa {
        playa = playa.with_options(self.to_lib_options());

        if self.meta {
            playa = playa.show_meta();
        }

        if self.force_host {
            playa = playa.force_host();
        }

        #[cfg(feature = "audio-ducking")]
        if !self.no_duck {
            if let Ok(config) = DuckConfig::new(self.duck_ramp_ms, self.duck_floor) {
                playa = playa.with_ducked_audio(config);
            }
        }

        playa
    }

    #[cfg(feature = "audio-ducking")]
    fn has_ducking(&self) -> bool {
        !self.no_duck
    }
}

fn background_requested(cli: &Cli) -> bool {
    cli.playback.background
        || match &cli.command {
            Some(Command::Play { playback, .. }) => playback.background,
            Some(Command::Effect { playback, .. }) => playback.background,
            _ => false,
        }
}

fn has_playback_target(cli: &Cli) -> bool {
    match &cli.command {
        Some(Command::Play { .. }) | Some(Command::Effect { .. }) => true,
        None => cli.audio_file.is_some(),
        _ => false,
    }
}

fn spawn_background_process() -> Result<(), std::io::Error> {
    let args: Vec<String> = std::env::args()
        .filter(|arg| arg != "--background")
        .collect();
    let Some(program) = args.first() else {
        return Err(std::io::Error::other(
            "failed to determine executable for background playback",
        ));
    };

    std::process::Command::new(program)
        .args(&args[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    Ok(())
}

fn maybe_spawn_background(cli: &Cli) -> bool {
    if !background_requested(cli) {
        return false;
    }

    if !has_playback_target(cli) {
        error_exit(
            "--background can only be used when playing an audio file or sound effect",
            2,
        );
    }

    if let Err(error) = spawn_background_process() {
        error_exit(&format!("failed to start background playback: {error}"), 1);
    }

    true
}

#[cfg(feature = "audio-ducking")]
#[tokio::main]
async fn main() {
    run_cli().await;
}

#[cfg(not(feature = "audio-ducking"))]
fn main() {
    run_cli_sync();
}

#[cfg(feature = "audio-ducking")]
async fn run_cli() {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    if maybe_spawn_background(&cli) {
        return;
    }

    match cli.command {
        Some(Command::ListEffects { filter }) => {
            list_sound_effects(filter.as_deref());
        }
        Some(Command::Players) => {
            let (markdown, missing) = build_metadata_markdown();
            render_markdown(&markdown, &missing);
        }
        #[cfg(feature = "sfx-native")]
        Some(Command::OutputChannels) => {
            list_output_channels();
        }
        Some(Command::DuckInfo) => {
            print_duck_info().await;
        }
        Some(Command::Effect { name, playback }) => {
            play_effect(&name, &playback).await;
        }
        Some(Command::Play {
            audio_file,
            playback,
        }) => {
            play_file(&audio_file, &playback).await;
        }
        None => {
            // Default: play the audio file if provided
            if let Some(ref audio_file) = cli.audio_file {
                play_file(audio_file, &cli.playback).await;
            } else {
                // No subcommand and no file - show help
                let _ = Cli::command().print_help();
                println!();
            }
        }
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn run_cli_sync() {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    if maybe_spawn_background(&cli) {
        return;
    }

    match cli.command {
        Some(Command::ListEffects { filter }) => {
            list_sound_effects(filter.as_deref());
        }
        Some(Command::Players) => {
            let (markdown, missing) = build_metadata_markdown();
            render_markdown(&markdown, &missing);
        }
        #[cfg(feature = "sfx-native")]
        Some(Command::OutputChannels) => {
            list_output_channels();
        }
        Some(Command::Effect { name, playback }) => {
            play_effect_sync(&name, &playback);
        }
        Some(Command::Play {
            audio_file,
            playback,
        }) => {
            play_file_sync(&audio_file, &playback);
        }
        None => {
            // Default: play the audio file if provided
            if let Some(ref audio_file) = cli.audio_file {
                play_file_sync(audio_file, &cli.playback);
            } else {
                // No subcommand and no file - show help
                let _ = Cli::command().print_help();
                println!();
            }
        }
    }
}

#[cfg(feature = "audio-ducking")]
async fn play_file(path: &PathBuf, opts: &PlaybackOptions) {
    let playa = match Playa::from_path(path) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to detect audio format: {error}"), 1);
        }
    };

    if opts.has_ducking() {
        if let Err(error) = playa.play_async().await {
            error_exit(&format!("playback failed: {error}"), 1);
        }
    } else if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

#[cfg(feature = "audio-ducking")]
async fn play_effect(name: &str, opts: &PlaybackOptions) {
    let Some(effect) = SoundEffect::from_name(name) else {
        error_exit(
            &format!("unknown sound effect: {name}. Use `playa list-effects` to see available effects"),
            2,
        );
    };

    // Use native SFX playback when available (with OS audio channel routing).
    // Ducking is set up before SFX playback so both paths benefit.
    #[cfg(feature = "sfx-native")]
    if !opts.force_host {
        #[cfg(feature = "audio-ducking")]
        let guard = if opts.has_ducking() {
            let backend = create_backend();
            match DuckConfig::new(opts.duck_ramp_ms, opts.duck_floor) {
                Ok(config) => DuckGuard::new(backend, config).await.ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        if playa::sfx_player::play_sfx(effect.bytes(), &opts.to_lib_options()).is_ok() {
            #[cfg(feature = "audio-ducking")]
            if let Some(guard) = guard {
                guard.restore().await;
            }
            return;
        }
        // Fall through to Playa builder path on error
    }

    let playa = match Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to load sound effect: {error}"), 1);
        }
    };

    if opts.has_ducking() {
        if let Err(error) = playa.play_async().await {
            error_exit(&format!("playback failed: {error}"), 1);
        }
    } else if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn play_file_sync(path: &PathBuf, opts: &PlaybackOptions) {
    let playa = match Playa::from_path(path) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to detect audio format: {error}"), 1);
        }
    };

    if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn play_effect_sync(name: &str, opts: &PlaybackOptions) {
    let Some(effect) = SoundEffect::from_name(name) else {
        error_exit(
            &format!("unknown sound effect: {name}. Use `playa list-effects` to see available effects"),
            2,
        );
    };

    // Use native SFX playback when available.
    #[cfg(feature = "sfx-native")]
    if !opts.force_host
        && playa::sfx_player::play_sfx(effect.bytes(), &opts.to_lib_options()).is_ok()
    {
        return;
    }

    let playa = match Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to load sound effect: {error}"), 1);
        }
    };

    if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

fn list_sound_effects(filter: Option<&str>) {
    let effects = SoundEffect::all();
    if effects.is_empty() {
        error_exit(
            "no sound effects are enabled in this build. Rebuild with `cargo build -p playa-cli --features sound-effects`",
            1,
        );
    }

    // Apply fuzzy filter: case-insensitive, strip non-alphanumeric, match against
    // name, description, and category
    let effects: Vec<SoundEffect> = match filter {
        Some(query) => {
            let normalized_query = fuzzy_normalize(query);
            effects
                .into_iter()
                .filter(|effect| {
                    fuzzy_normalize(effect.name()).contains(&normalized_query)
                        || fuzzy_normalize(effect.description()).contains(&normalized_query)
                        || fuzzy_normalize(effect.category()).contains(&normalized_query)
                })
                .collect()
        }
        None => effects,
    };

    if effects.is_empty() {
        error_exit(
            &format!("no effects match filter {:?}", filter.unwrap_or("")),
            1,
        );
    }

    // Group effects by category
    let mut categories: BTreeMap<&str, Vec<SoundEffect>> = BTreeMap::new();
    for effect in &effects {
        categories
            .entry(effect.category())
            .or_default()
            .push(*effect);
    }

    // Build the nested list structure
    let mut top_list = UnorderedList::empty();

    for (category, cat_effects) in categories {
        // Category header as a Prose with blue bold styling
        let header = Prose::new(format!("<blue><bold>{}</bold></blue>", category));
        top_list.add(header);

        // Build the inner list of effects
        let effect_items: Vec<RenderableContent> = cat_effects
            .iter()
            .map(|effect| {
                let styled = Prose::new(format!(
                    "{} <dim><italic>{}</italic></dim> [<dim>{}</dim>]",
                    effect.name(),
                    effect.description(),
                    format_duration(effect.duration_ms())
                ));
                RenderableContent::Component(Rc::new(styled))
            })
            .collect();

        let inner_list = UnorderedList::from(effect_items);
        top_list.add(inner_list);
    }

    // Render and print
    let output = top_list.render_optimistic(None);
    println!("{}", output);
}

/// Normalize a string for fuzzy matching: lowercase, strip non-alphanumeric.
fn fuzzy_normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn format_duration(duration_ms: Option<u32>) -> String {
    match duration_ms {
        Some(ms) if ms < 1000 => format!("{}ms", ms),
        Some(ms) => format!("<orange>{:.2}s</orange>", ms as f64 / 1000.0),
        None => "unknown".to_string(),
    }
}

#[cfg(feature = "audio-ducking")]
async fn print_duck_info() {
    let backend = create_backend();
    let name = backend_name();

    println!("Audio Ducking Backend Info");
    println!("==========================");
    println!("Selected backend: {}", name);
    println!(
        "Available: {}",
        if backend.is_available() { "yes" } else { "no" }
    );
    println!();

    match name {
        "macos-coreaudio" => {
            println!("Strategy: Volume control via CoreAudio");
            println!("  - Fades system audio volume down during playback");
            println!("  - Restores original volume after playback");
            println!("  - Works with devices that expose software volume control");
        }
        "macos-media-keys" => {
            println!("Strategy: Media key pause/resume (fallback)");
            println!("  - Detects if media is playing before pausing");
            println!("  - Only resumes if media was playing before ducking");
            println!("  - Used because your output device doesn't support software volume");
            println!();

            // Check if nowplaying-cli is available
            let has_nowplaying = std::process::Command::new("which")
                .arg("nowplaying-cli")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if has_nowplaying {
                println!("Detection: Using nowplaying-cli (all apps supported)");
            } else {
                println!("Detection: AppleScript fallback (Spotify, Music, TIDAL, VLC, Podcasts)");
                println!("  Tip: Install `nowplaying-cli` for browser/universal detection:");
                println!("       brew install nowplaying-cli");
            }
            println!();

            // Show current playback state
            let snapshot = backend.snapshot().await;
            match snapshot {
                Ok(snap) => {
                    let is_playing = snap
                        .entries
                        .first()
                        .and_then(|e| e.channels.first())
                        .map(|v| *v > 0.5)
                        .unwrap_or(false);
                    println!(
                        "Current state: {}",
                        if is_playing {
                            "Media is PLAYING"
                        } else {
                            "No media playing (or paused)"
                        }
                    );
                }
                Err(e) => {
                    println!("Could not detect playback state: {}", e);
                }
            }
        }
        "linux-pulse" => {
            println!("Strategy: Per-application volume control via PulseAudio/PipeWire");
            println!("  - Ducks individual applications (sink inputs)");
            println!("  - Excludes Playa's own audio from ducking");
            println!("  - Works with PipeWire's PulseAudio compatibility layer");
            println!();

            // Show current applications
            let snapshot = backend.snapshot().await;
            match snapshot {
                Ok(snap) => {
                    if snap.is_empty() {
                        println!("Current state: No other applications playing audio");
                    } else {
                        println!("Applications that would be ducked ({}):", snap.len());
                        for entry in &snap.entries {
                            if let playa::ducking::SessionId::PulseSinkInput { index, name } =
                                &entry.id
                            {
                                let vol = entry.channels.first().copied().unwrap_or(0.0) * 100.0;
                                println!("  [{}] {} - {:.0}%", index, name, vol);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Could not list applications: {}", e);
                }
            }
        }
        "linux-alsa" => {
            println!("Strategy: System-wide volume control via ALSA (fallback)");
            println!("  - Fades master volume down during playback");
            println!("  - Affects ALL audio including Playa's output");
            println!("  - Used because PulseAudio is not available");
            println!();
            println!("Tip: For per-application ducking, install PulseAudio or PipeWire");
        }
        "noop" => {
            println!("Strategy: No ducking (disabled or unavailable)");
            println!("  - Audio playback will not affect other audio sources");
        }
        _ => {
            println!("Strategy: {}", name);
        }
    }
}

fn build_metadata_markdown() -> (String, Vec<String>) {
    let installed = InstalledHeadlessAudio::new();
    let missing = collect_missing_players(&installed);
    let markdown = build_metadata_markdown_table(&installed);
    (markdown, missing)
}

fn build_metadata_markdown_table(installed: &InstalledHeadlessAudio) -> String {
    let mut lines = Vec::new();
    lines.push("| I | Software | Codec Support | File Formats |".to_string());
    lines.push("|---|---|---|---|".to_string());

    for player in all_players() {
        let Some(metadata) = PLAYER_LOOKUP.get(player) else {
            continue;
        };
        let is_installed = installed.is_installed(player.as_headless_audio());
        let indicator = if is_installed { "\u{2705}" } else { "\u{274c}" };
        // Only show clickable links for installed players
        let software = if is_installed {
            link_for_player(*player)
        } else {
            display_name_for_player(*player)
        };
        let codecs = escape_markdown_cell(&format_codec_list(metadata.supported_codecs));
        let formats = escape_markdown_cell(&format_format_list(metadata.supported_formats));
        lines.push(format!(
            "| {} | {} | {} | {} |",
            indicator, software, codecs, formats
        ));
    }

    lines.join("\n")
}

fn collect_missing_players(installed: &InstalledHeadlessAudio) -> Vec<String> {
    all_players()
        .iter()
        .filter(|player| !installed.is_installed(player.as_headless_audio()))
        .map(|player| display_name_for_player(*player))
        .collect()
}

fn link_for_player(player: AudioPlayer) -> String {
    PLAYER_LOOKUP
        .get(&player)
        .map(|metadata| {
            let website = metadata.website().trim();
            if website.is_empty() {
                metadata.display_name().to_string()
            } else {
                format!("[{}]({})", metadata.display_name(), website)
            }
        })
        .unwrap_or_else(|| format!("{player:?}"))
}

fn display_name_for_player(player: AudioPlayer) -> String {
    PLAYER_LOOKUP
        .get(&player)
        .map(|metadata| metadata.display_name().to_string())
        .unwrap_or_else(|| format!("{player:?}"))
}

fn format_codec_list(codecs: &[Codec]) -> String {
    if codecs.is_empty() {
        return "None".to_string();
    }
    codecs
        .iter()
        .map(|codec| format_codec_label(*codec))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_format_list(formats: &[AudioFileFormat]) -> String {
    if formats.is_empty() {
        return "None".to_string();
    }
    formats
        .iter()
        .map(|format| format_format_label(*format))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_codec_label(codec: Codec) -> String {
    match codec {
        Codec::Pcm => "PCM".to_string(),
        Codec::Flac => "FLAC".to_string(),
        Codec::Alac => "ALAC".to_string(),
        Codec::Mp3 => "MP3".to_string(),
        Codec::Aac => "AAC".to_string(),
        Codec::Vorbis => "Vorbis".to_string(),
        Codec::Opus => "Opus".to_string(),
    }
}

fn format_format_label(format: AudioFileFormat) -> String {
    match format {
        AudioFileFormat::Wav => ".wav".to_string(),
        AudioFileFormat::Aiff => ".aiff".to_string(),
        AudioFileFormat::Flac => ".flac".to_string(),
        AudioFileFormat::Mp3 => ".mp3".to_string(),
        AudioFileFormat::Ogg => ".ogg".to_string(),
        AudioFileFormat::M4a => ".m4a".to_string(),
        AudioFileFormat::Webm => ".webm".to_string(),
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn render_markdown(content: &str, missing_players: &[String]) {
    let markdown = Markdown::from(content.to_string());
    match for_terminal(&markdown, TerminalOptions::default()) {
        Ok(rendered) => {
            let output = dim_missing_rows(&rendered, missing_players);
            print!("{}", output);
        }
        Err(_) => println!("{}", markdown.content()),
    }
    append_native_playback_note();
}

fn append_native_playback_note() {
    #[cfg(feature = "native-playback")]
    {
        let mut list = UnorderedList::empty();
        list.add(Prose::new(
            "<italic>native playback is enabled and will be used for \
             .wav, .aiff, .mp3, .flac, .m4a, .ogg, .webm; \
             falling back to host players where needed</italic>",
        ));
        print!("{}", list.render_optimistic(None));
    }
    #[cfg(all(feature = "sfx-native", not(feature = "native-playback")))]
    {
        let mut list = UnorderedList::empty();
        list.add(Prose::new(
            "<italic>native playback is enabled for .wav, .mp3, .ogg; \
             falling back to host players where needed</italic>",
        ));
        print!("{}", list.render_optimistic(None));
    }
}

fn dim_missing_rows(rendered: &str, missing_players: &[String]) -> String {
    if missing_players.is_empty() {
        return rendered.to_string();
    }

    let mut output = String::with_capacity(rendered.len() + missing_players.len() * 12);
    let mut current_row_missing = false;

    for line_with_newline in rendered.split_inclusive('\n') {
        let (line, newline) = line_with_newline
            .strip_suffix('\n')
            .map(|line| (line, "\n"))
            .unwrap_or((line_with_newline, ""));

        let plain_line = strip_osc8_sequences(&strip_ansi_codes(line));

        if line.starts_with(TABLE_DIVIDER) {
            if let Some(cell) = software_table_cell(&plain_line) {
                let trimmed = cell.trim();
                if !trimmed.is_empty() {
                    current_row_missing =
                        missing_players.iter().any(|name| trimmed.starts_with(name));
                }
            }

            if current_row_missing {
                output.push_str(&dim_table_row_line(line));
            } else {
                output.push_str(line);
            }
        } else {
            current_row_missing = false;
            output.push_str(line);
        }

        output.push_str(newline);
    }

    output
}

fn dim_table_row_line(line: &str) -> String {
    // Split by table divider, strip all ANSI/OSC8 from cells 2+ (skip I column),
    // and replace with uniform grey. This prevents darkmatter's inline colors
    // (text color, hyperlink blue) from overriding the dim effect.
    let parts: Vec<&str> = line.split(TABLE_DIVIDER).collect();
    let mut output = String::with_capacity(line.len() + 32);

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            output.push(TABLE_DIVIDER);
        }
        // Cells 0 (before first │) and 1 (I column) stay untouched
        if i >= 2 && i < parts.len() - 1 {
            let stripped = strip_osc8_sequences(&strip_ansi_codes(part));
            output.push_str(MISSING_FG);
            output.push_str(&stripped);
            output.push_str(RESET);
        } else {
            output.push_str(part);
        }
    }

    output
}

/// Returns the second table cell (Software column, skipping the I column).
fn software_table_cell(line: &str) -> Option<&str> {
    let mut parts = line.split(TABLE_DIVIDER);
    parts.next()?; // before first divider
    parts.next()?; // I column
    parts.next() // Software column
}

fn strip_osc8_sequences(input: &str) -> String {
    let osc8_start = "\x1b]8;;";
    let osc8_end = "\x1b]8;;\x07";
    let mut output = String::new();
    let mut remaining = input;

    while let Some(start) = remaining.find(osc8_start) {
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start + osc8_start.len()..];
        let Some(bel_pos) = after_start.find('\x07') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let after_url = &after_start[bel_pos + 1..];
        let Some(end_pos) = after_url.find(osc8_end) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let display = &after_url[..end_pos];
        output.push_str(display);
        remaining = &after_url[end_pos + osc8_end.len()..];
    }

    output.push_str(remaining);
    output
}

#[cfg(feature = "sfx-native")]
fn list_output_channels() {
    match playa::get_output_channels() {
        Ok(channels) => {
            if channels.is_empty() {
                println!("No native audio output channels found.");
                return;
            }

            println!("Available Output Channels");
            println!("=========================");
            println!();

            let mut list = UnorderedList::empty();
            for channel in channels {
                let mut styled_name = channel.name.clone();
                let mut markers = Vec::new();

                if channel.is_default_audio && channel.is_default_sfx {
                    styled_name = format!("<bold><italic>{}</italic></bold>", styled_name);
                } else if channel.is_default_audio {
                    styled_name = format!("<bold>{}</bold>", styled_name);
                } else if channel.is_default_sfx {
                    styled_name = format!("<italic>{}</italic>", styled_name);
                }

                if channel.is_default_audio {
                    markers.push("default audio");
                }
                if channel.is_default_sfx {
                    markers.push("default sfx");
                }

                let text = if markers.is_empty() {
                    format!("{} <dim>[{}]</dim>", styled_name, channel.id)
                } else {
                    format!(
                        "{} <dim>[{}] ({})</dim>",
                        styled_name,
                        channel.id,
                        markers.join(", ")
                    )
                };

                list.add(Prose::new(text));
            }
            let output = list.render_optimistic(None);
            print!("{}", output);
        }
        Err(e) => {
            error_exit(&format!("failed to get output channels: {e}"), 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_meta_markdown_with_formatting_and_links() {
        let installed = InstalledHeadlessAudio::new();
        let markdown = build_metadata_markdown_table(&installed);

        assert!(markdown.contains("| I | Software | Codec Support | File Formats |"));
        assert!(markdown.contains("PCM"));
        assert!(markdown.contains("Vorbis"));
        assert!(markdown.contains(".wav"));
        assert!(markdown.contains(&link_for_player(AudioPlayer::Mpv)));
        // Every row has an installed indicator
        assert!(markdown.contains('\u{2705}') || markdown.contains('\u{274c}'));
    }

    #[test]
    fn playback_options_default() {
        let opts = PlaybackOptions {
            meta: false,
            background: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        };
        assert!(!opts.fast);
        assert!(!opts.slow);
        assert!(!opts.quiet);
        assert!(!opts.loud);
        assert!(opts.speed.is_none());
        assert!(opts.volume.is_none());
    }

    #[test]
    fn playback_options_fast() {
        let opts = PlaybackOptions {
            meta: false,
            background: false,
            fast: true,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        };
        assert!(opts.fast);
    }

    #[test]
    fn playback_options_custom_speed_and_volume() {
        let opts = PlaybackOptions {
            meta: false,
            background: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: Some(0.9),
            volume: Some(0.3),
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        };
        assert_eq!(opts.speed, Some(0.9));
        assert_eq!(opts.volume, Some(0.3));
    }

    fn base_playback_options() -> PlaybackOptions {
        PlaybackOptions {
            meta: false,
            background: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        }
    }

    #[test]
    fn background_flag_detected_for_playback_target() {
        let cli = Cli {
            command: Some(Command::Play {
                audio_file: PathBuf::from("tone.wav"),
                playback: PlaybackOptions {
                    background: true,
                    ..base_playback_options()
                },
            }),
            audio_file: None,
            playback: base_playback_options(),
        };

        assert!(background_requested(&cli));
        assert!(has_playback_target(&cli));
    }

    #[test]
    fn background_flag_rejected_without_playback_target() {
        let cli = Cli {
            command: Some(Command::Players),
            audio_file: None,
            playback: PlaybackOptions {
                background: true,
                channel: None,
                ..base_playback_options()
            },
        };

        assert!(background_requested(&cli));
        assert!(!has_playback_target(&cli));
    }
}
