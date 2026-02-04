use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::CompleteEnv;
use sniff_lib::programs::InstalledHeadlessAudio;

use playa::{AudioFileFormat, AudioPlayer, Codec, PLAYER_LOOKUP, Playa, SoundEffect, all_players};

use darkmatter_lib::markdown::Markdown;
use darkmatter_lib::markdown::output::terminal::{TerminalOptions, for_terminal};
use darkmatter_lib::testing::strip_ansi_codes;
#[cfg(feature = "audio-ducking")]
use playa::ducking::{DuckConfig, backend_name, create_backend};

const MISSING_FG: &str = "\x1b[38;2;140;140;140m";
const ITALIC: &str = "\x1b[3m";
const RESET: &str = "\x1b[0m";
const TABLE_DIVIDER: char = '\u{2502}';

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
        #[arg(value_name = "NAME")]
        name: String,

        #[command(flatten)]
        playback: PlaybackOptions,
    },

    /// List available built-in sound effects
    ListEffects,

    /// Show a table of available audio players
    Players,

    /// Show audio ducking backend info
    #[cfg(feature = "audio-ducking")]
    DuckInfo,
}

/// Playback options shared between play and effect commands
#[derive(Parser, Clone)]
struct PlaybackOptions {
    /// Display playback metadata (player, volume, speed, codec, format)
    #[arg(long)]
    meta: bool,

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
    #[arg(long, value_name = "LEVEL", conflicts_with_all = ["quiet", "loud"])]
    volume: Option<f32>,

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
    fn apply_to_playa(&self, mut playa: Playa) -> Playa {
        if let Some(speed) = self.speed {
            playa = playa.speed(speed);
        } else if self.fast {
            playa = playa.speed(1.25);
        } else if self.slow {
            playa = playa.speed(0.75);
        }

        if let Some(volume) = self.volume {
            playa = playa.volume(volume);
        } else if self.quiet {
            playa = playa.volume(0.5);
        } else if self.loud {
            playa = playa.volume(1.5);
        }

        if self.meta {
            playa = playa.show_meta();
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

    match cli.command {
        Some(Command::ListEffects) => {
            list_sound_effects();
        }
        Some(Command::Players) => {
            let (markdown, missing) = build_metadata_markdown();
            render_markdown(&markdown, &missing);
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

    match cli.command {
        Some(Command::ListEffects) => {
            list_sound_effects();
        }
        Some(Command::Players) => {
            let (markdown, missing) = build_metadata_markdown();
            render_markdown(&markdown, &missing);
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
            eprintln!("Failed to detect audio format: {error}");
            std::process::exit(1);
        }
    };

    if opts.has_ducking() {
        if let Err(error) = playa.play_async().await {
            eprintln!("Playback failed: {error}");
            std::process::exit(1);
        }
    } else if let Err(error) = playa.play() {
        eprintln!("Playback failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(feature = "audio-ducking")]
async fn play_effect(name: &str, opts: &PlaybackOptions) {
    let Some(effect) = SoundEffect::from_name(name) else {
        eprintln!(
            "Unknown sound effect: {name}. Use `playa list-effects` to see available effects."
        );
        std::process::exit(2);
    };

    let playa = match Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            eprintln!("Failed to load sound effect: {error}");
            std::process::exit(1);
        }
    };

    if opts.has_ducking() {
        if let Err(error) = playa.play_async().await {
            eprintln!("Playback failed: {error}");
            std::process::exit(1);
        }
    } else if let Err(error) = playa.play() {
        eprintln!("Playback failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn play_file_sync(path: &PathBuf, opts: &PlaybackOptions) {
    let playa = match Playa::from_path(path) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            eprintln!("Failed to detect audio format: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = playa.play() {
        eprintln!("Playback failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn play_effect_sync(name: &str, opts: &PlaybackOptions) {
    let Some(effect) = SoundEffect::from_name(name) else {
        eprintln!(
            "Unknown sound effect: {name}. Use `playa list-effects` to see available effects."
        );
        std::process::exit(2);
    };

    let playa = match Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            eprintln!("Failed to load sound effect: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = playa.play() {
        eprintln!("Playback failed: {error}");
        std::process::exit(1);
    }
}

fn list_sound_effects() {
    let effects = SoundEffect::all();
    if effects.is_empty() {
        eprintln!(
            "No sound effects are enabled in this build. Rebuild with `cargo build -p playa-cli --features sound-effects`."
        );
        std::process::exit(1);
    }

    println!("Available sound effects ({}):", effects.len());
    for effect in effects {
        println!("  {}", effect.name());
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
    let markdown = build_metadata_markdown_table();
    (markdown, missing)
}

fn build_metadata_markdown_table() -> String {
    let mut lines = Vec::new();
    lines.push("| Software | Codec Support | File Formats |".to_string());
    lines.push("|---|---|---|".to_string());

    for player in all_players() {
        let Some(metadata) = PLAYER_LOOKUP.get(player) else {
            continue;
        };
        let software = link_for_player(*player);
        let codecs = escape_markdown_cell(&format_codec_list(metadata.supported_codecs));
        let formats = escape_markdown_cell(&format_format_list(metadata.supported_formats));
        lines.push(format!("| {} | {} | {} |", software, codecs, formats));
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
            let mut output = dim_missing_rows(&rendered, missing_players);
            append_missing_note(&mut output);
            print!("{}", output);
        }
        Err(_) => println!("{}", markdown.content()),
    }
}

fn append_missing_note(output: &mut String) {
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(&format!(
        "- {italic}items listed in {grey}grey{reset}{italic} are not installed{reset}\n",
        italic = ITALIC,
        grey = MISSING_FG,
        reset = RESET
    ));
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
            if let Some(cell) = first_table_cell(&plain_line) {
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
    let mut output = String::with_capacity(line.len() + 16);
    let mut in_cell = false;

    for ch in line.chars() {
        if ch == TABLE_DIVIDER {
            if in_cell {
                output.push_str(RESET);
            }
            output.push(ch);
            output.push_str(MISSING_FG);
            in_cell = true;
        } else {
            output.push(ch);
        }
    }

    if in_cell {
        output.push_str(RESET);
    }

    output
}

fn first_table_cell(line: &str) -> Option<&str> {
    let mut parts = line.split(TABLE_DIVIDER);
    parts.next()?;
    parts.next()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_meta_markdown_with_formatting_and_links() {
        let markdown = build_metadata_markdown_table();

        assert!(markdown.contains("| Software | Codec Support | File Formats |"));
        assert!(markdown.contains("PCM"));
        assert!(markdown.contains("Vorbis"));
        assert!(markdown.contains(".wav"));
        assert!(markdown.contains(&link_for_player(AudioPlayer::Mpv)));
    }

    #[test]
    fn playback_options_default() {
        let opts = PlaybackOptions {
            meta: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
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
            fast: true,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
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
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: Some(0.9),
            volume: Some(0.3),
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
}
